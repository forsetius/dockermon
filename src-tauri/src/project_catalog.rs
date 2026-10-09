use crate::runtime::{
    Locale, Preferences, ProjectProfileSnapshot, ProjectSnapshot, RuntimeError, ServiceSnapshot,
    ServiceStatus, ThemePreference,
};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::process::Command;

const CONFIG_SCHEMA_VERSION: u32 = 1;
const CONFIG_DIRECTORY_NAME: &str = "dockermon";
const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredProject {
    pub compose_files: Vec<PathBuf>,
    pub id: String,
    pub name: String,
    pub running_services: BTreeSet<String>,
    pub working_directory: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComposeExecutionContext {
    pub compose_files: Vec<PathBuf>,
    pub enabled_profiles: Vec<String>,
    pub working_directory: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ComposeProjectSource {
    compose_files: Vec<PathBuf>,
    working_directory: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ComposeProjectDefinition {
    id: String,
    name: String,
    services: Vec<ComposeServiceDefinition>,
    source: ComposeProjectSource,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct ComposeServiceDefinition {
    name: String,
    #[serde(default)]
    profiles: Vec<String>,
}

trait ComposeProjectReader: Send + Sync {
    fn inspect<'a>(
        &'a self,
        source: &'a ComposeProjectSource,
    ) -> BoxFuture<'a, Result<ComposeProjectDefinition, RuntimeError>>;
}

struct DockerComposeReader;

impl ComposeProjectReader for DockerComposeReader {
    fn inspect<'a>(
        &'a self,
        source: &'a ComposeProjectSource,
    ) -> BoxFuture<'a, Result<ComposeProjectDefinition, RuntimeError>> {
        Box::pin(async move {
            let mut command = Command::new("docker");
            command.arg("compose");
            for compose_file in &source.compose_files {
                command.arg("-f").arg(compose_file);
            }
            let output = command
                .arg("--profile")
                .arg("*")
                .arg("config")
                .arg("--format")
                .arg("json")
                .current_dir(&source.working_directory)
                .output()
                .await
                .map_err(|error| {
                    if error.kind() == std::io::ErrorKind::NotFound {
                        RuntimeError::new("COMPOSE_CLI_NOT_FOUND", false)
                    } else {
                        RuntimeError::new("COMPOSE_CONFIG_UNAVAILABLE", true)
                    }
                })?;

            if !output.status.success() {
                return Err(RuntimeError::new("COMPOSE_CONFIG_INVALID", false));
            }

            parse_compose_config(&output.stdout, source.clone())
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogConfiguration {
    #[serde(default)]
    ignored_project_ids: BTreeSet<String>,
    preferences: StoredPreferences,
    project_settings: BTreeMap<String, StoredProjectSettings>,
    projects: Vec<StoredProject>,
    schema_version: u32,
}

impl Default for CatalogConfiguration {
    fn default() -> Self {
        Self {
            ignored_project_ids: BTreeSet::new(),
            preferences: StoredPreferences::default(),
            project_settings: BTreeMap::new(),
            projects: Vec::new(),
            schema_version: CONFIG_SCHEMA_VERSION,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct StoredPreferences {
    language: Locale,
    theme: ThemePreference,
}

impl Default for StoredPreferences {
    fn default() -> Self {
        Self {
            language: Locale::system(),
            theme: ThemePreference::System,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredProjectSettings {
    #[serde(default)]
    active: bool,
    #[serde(default)]
    bulk_selection: BTreeMap<String, bool>,
    #[serde(default)]
    enabled_profiles: Vec<String>,
    #[serde(default)]
    profiles_configured: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredProject {
    compose_files: Vec<String>,
    id: String,
    name: String,
    services: Vec<ComposeServiceDefinition>,
    working_directory: String,
}

impl From<ComposeProjectDefinition> for StoredProject {
    fn from(definition: ComposeProjectDefinition) -> Self {
        Self {
            compose_files: definition
                .source
                .compose_files
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            id: definition.id,
            name: definition.name,
            services: definition.services,
            working_directory: definition
                .source
                .working_directory
                .to_string_lossy()
                .into_owned(),
        }
    }
}

pub struct ProjectCatalog {
    configuration: CatalogConfiguration,
    configuration_path: PathBuf,
    load_error: Option<RuntimeError>,
    reader: Arc<dyn ComposeProjectReader>,
}

impl ProjectCatalog {
    pub fn load_default() -> Self {
        let configuration_path = default_configuration_path();
        Self::load(configuration_path, Arc::new(DockerComposeReader))
    }

    fn load(configuration_path: PathBuf, reader: Arc<dyn ComposeProjectReader>) -> Self {
        match read_configuration(&configuration_path) {
            Ok(configuration) => Self {
                configuration,
                configuration_path,
                load_error: None,
                reader,
            },
            Err(error) => Self {
                configuration: CatalogConfiguration::default(),
                configuration_path,
                load_error: Some(error),
                reader,
            },
        }
    }

    pub fn preferences(&self) -> Preferences {
        Preferences {
            language: self.configuration.preferences.language,
            theme: self.configuration.preferences.theme,
        }
    }

    pub(crate) fn execution_context(
        &self,
        project_id: &str,
    ) -> Result<ComposeExecutionContext, RuntimeError> {
        let project = self
            .configuration
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| RuntimeError::new("PROJECT_SOURCE_UNAVAILABLE", false))?;
        let settings = self
            .configuration
            .project_settings
            .get(project_id)
            .cloned()
            .unwrap_or_default();

        Ok(ComposeExecutionContext {
            compose_files: project.compose_files.iter().map(PathBuf::from).collect(),
            enabled_profiles: settings.enabled_profiles,
            working_directory: PathBuf::from(&project.working_directory),
        })
    }

    pub async fn import_project(&mut self, paths: Vec<String>) -> Result<(), RuntimeError> {
        self.ensure_writable()?;
        let source = normalize_project_source(paths)?;
        let definition = self.reader.inspect(&source).await?;
        self.configuration
            .ignored_project_ids
            .remove(&definition.id);
        self.upsert_project(definition);
        self.save()
    }

    pub fn remove_project(&mut self, project_id: &str) -> Result<(), RuntimeError> {
        self.ensure_writable()?;
        self.configuration
            .projects
            .retain(|project| project.id != project_id);
        self.configuration.project_settings.remove(project_id);
        self.configuration
            .ignored_project_ids
            .insert(project_id.to_owned());
        self.save()
    }

    pub async fn merge_projects(
        &mut self,
        runtime_projects: Vec<ProjectSnapshot>,
        discoveries: Vec<DiscoveredProject>,
    ) -> Vec<ProjectSnapshot> {
        let mut configuration_changed = false;
        if self.load_error.is_none() {
            for discovery in discoveries {
                if self
                    .configuration
                    .ignored_project_ids
                    .contains(&discovery.id)
                {
                    continue;
                }
                let stored_project = self
                    .configuration
                    .projects
                    .iter()
                    .find(|project| project.id == discovery.id);
                let source_changed = stored_project.is_some_and(|project| {
                    project.working_directory != discovery.working_directory.to_string_lossy()
                        || project.compose_files
                            != discovery
                                .compose_files
                                .iter()
                                .map(|path| path.to_string_lossy().into_owned())
                                .collect::<Vec<_>>()
                });
                if stored_project.is_none() || source_changed {
                    let source = ComposeProjectSource {
                        compose_files: discovery.compose_files.clone(),
                        working_directory: discovery.working_directory.clone(),
                    };
                    if let Ok(definition) = self.reader.inspect(&source).await {
                        self.upsert_project(definition);
                        configuration_changed = true;
                    }
                }

                if let Some(project) = self
                    .configuration
                    .projects
                    .iter()
                    .find(|project| project.id == discovery.id)
                {
                    let settings = self
                        .configuration
                        .project_settings
                        .entry(discovery.id.clone())
                        .or_default();
                    if settings.profiles_configured {
                        continue;
                    }
                    let enabled_profile_count = settings.enabled_profiles.len();
                    for service in &project.services {
                        if discovery.running_services.contains(&service.name) {
                            for profile in &service.profiles {
                                if !settings.enabled_profiles.contains(profile) {
                                    settings.enabled_profiles.push(profile.clone());
                                }
                            }
                        }
                    }
                    settings.enabled_profiles.sort();
                    settings.enabled_profiles.dedup();
                    configuration_changed |=
                        settings.enabled_profiles.len() != enabled_profile_count;
                }
            }
        }

        if configuration_changed {
            let _ = self.save();
        }

        self.decorate_projects(runtime_projects)
    }

    pub fn set_bulk_selected(
        &mut self,
        project_id: &str,
        service_name: &str,
        selected: bool,
    ) -> Result<(), RuntimeError> {
        self.ensure_writable()?;
        self.configuration
            .project_settings
            .entry(project_id.to_owned())
            .or_default()
            .bulk_selection
            .insert(service_name.to_owned(), selected);
        self.save()
    }

    pub fn set_language(&mut self, language: Locale) -> Result<(), RuntimeError> {
        self.ensure_writable()?;
        self.configuration.preferences.language = language;
        self.save()
    }

    pub fn set_project_active(
        &mut self,
        project_id: &str,
        active: bool,
    ) -> Result<(), RuntimeError> {
        self.ensure_writable()?;
        self.configuration
            .project_settings
            .entry(project_id.to_owned())
            .or_default()
            .active = active;
        self.save()
    }

    pub fn set_project_profiles(
        &mut self,
        project_id: &str,
        profiles: Vec<String>,
    ) -> Result<(), RuntimeError> {
        self.ensure_writable()?;
        let available_profiles = self.available_profiles(project_id);
        if profiles
            .iter()
            .any(|profile| !available_profiles.contains(profile))
        {
            return Err(RuntimeError::new("PROFILE_NOT_FOUND", false));
        }

        let mut profiles = profiles;
        profiles.sort();
        profiles.dedup();
        self.configuration
            .project_settings
            .entry(project_id.to_owned())
            .or_default()
            .enabled_profiles = profiles;
        self.configuration
            .project_settings
            .entry(project_id.to_owned())
            .or_default()
            .profiles_configured = true;
        self.save()
    }

    pub fn set_theme(&mut self, theme: ThemePreference) -> Result<(), RuntimeError> {
        self.ensure_writable()?;
        self.configuration.preferences.theme = theme;
        self.save()
    }

    pub fn refresh_project(&self, project: &mut ProjectSnapshot) {
        let settings = self
            .configuration
            .project_settings
            .get(&project.id)
            .cloned()
            .unwrap_or_default();
        let definition = self
            .configuration
            .projects
            .iter()
            .find(|definition| definition.id == project.id);
        decorate_project(project, definition, &settings);
    }

    pub(crate) fn decorate_projects(
        &self,
        runtime_projects: Vec<ProjectSnapshot>,
    ) -> Vec<ProjectSnapshot> {
        let mut projects: BTreeMap<String, ProjectSnapshot> = runtime_projects
            .into_iter()
            .filter(|project| !self.configuration.ignored_project_ids.contains(&project.id))
            .map(|project| (project.id.clone(), project))
            .collect();

        for definition in &self.configuration.projects {
            projects
                .entry(definition.id.clone())
                .or_insert_with(|| ProjectSnapshot {
                    active: false,
                    id: definition.id.clone(),
                    name: definition.name.clone(),
                    profiles: Vec::new(),
                    services: Vec::new(),
                });
        }

        for project in projects.values_mut() {
            self.refresh_project(project);
        }

        let mut projects: Vec<_> = projects.into_values().collect();
        projects.sort_by(|left, right| left.name.cmp(&right.name));
        projects
    }

    fn upsert_project(&mut self, definition: ComposeProjectDefinition) {
        let stored = StoredProject::from(definition);
        if let Some(existing) = self
            .configuration
            .projects
            .iter_mut()
            .find(|project| project.id == stored.id)
        {
            *existing = stored;
        } else {
            self.configuration.projects.push(stored);
            self.configuration
                .projects
                .sort_by(|left, right| left.name.cmp(&right.name));
        }
    }

    fn available_profiles(&self, project_id: &str) -> BTreeSet<String> {
        self.configuration
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .into_iter()
            .flat_map(|project| &project.services)
            .flat_map(|service| service.profiles.iter().cloned())
            .collect()
    }

    fn ensure_writable(&self) -> Result<(), RuntimeError> {
        match &self.load_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    fn save(&self) -> Result<(), RuntimeError> {
        self.ensure_writable()?;
        write_configuration(&self.configuration_path, &self.configuration)
    }
}

fn decorate_project(
    project: &mut ProjectSnapshot,
    definition: Option<&StoredProject>,
    settings: &StoredProjectSettings,
) {
    project.active = settings.active;
    let service_definitions: BTreeMap<_, _> = definition
        .into_iter()
        .flat_map(|definition| &definition.services)
        .map(|service| (service.name.as_str(), service))
        .collect();

    if let Some(definition) = definition {
        project.name.clone_from(&definition.name);
        for service in &definition.services {
            if project
                .services
                .iter()
                .all(|entry| entry.name != service.name)
            {
                project.services.push(ServiceSnapshot {
                    bulk_selected: true,
                    cpu_percent: None,
                    id: format!("{}:{}", project.id, service.name),
                    included: false,
                    memory_bytes: None,
                    name: service.name.clone(),
                    ports: Vec::new(),
                    profiles: service.profiles.clone(),
                    status: ServiceStatus::NotCreated,
                });
            }
        }
    }

    let available_profiles: BTreeSet<_> = service_definitions
        .values()
        .flat_map(|service| service.profiles.iter().cloned())
        .collect();
    project.profiles = available_profiles
        .into_iter()
        .map(|name| ProjectProfileSnapshot {
            enabled: settings.enabled_profiles.contains(&name),
            name,
        })
        .collect();

    for service in &mut project.services {
        service.profiles = service_definitions
            .get(service.name.as_str())
            .map(|definition| definition.profiles.clone())
            .unwrap_or_default();
        service.included = service.profiles.is_empty()
            || service
                .profiles
                .iter()
                .any(|profile| settings.enabled_profiles.contains(profile));
        service.bulk_selected = settings
            .bulk_selection
            .get(&service.name)
            .copied()
            .unwrap_or(service.bulk_selected);
    }
    project
        .services
        .sort_by(|left, right| left.name.cmp(&right.name));
}

#[derive(Deserialize)]
struct ComposeConfigurationDocument {
    name: String,
    services: BTreeMap<String, ComposeServiceDocument>,
}

#[derive(Deserialize)]
struct ComposeServiceDocument {
    #[serde(default)]
    profiles: Vec<String>,
}

fn parse_compose_config(
    json: &[u8],
    source: ComposeProjectSource,
) -> Result<ComposeProjectDefinition, RuntimeError> {
    let document: ComposeConfigurationDocument = serde_json::from_slice(json)
        .map_err(|_| RuntimeError::new("COMPOSE_CONFIG_INVALID", false))?;
    if document.name.trim().is_empty() || document.services.is_empty() {
        return Err(RuntimeError::new("COMPOSE_CONFIG_INVALID", false));
    }

    let name = document.name;
    Ok(ComposeProjectDefinition {
        id: format!("compose:{name}"),
        name,
        services: document
            .services
            .into_iter()
            .map(|(name, mut service)| {
                service.profiles.sort();
                service.profiles.dedup();
                ComposeServiceDefinition {
                    name,
                    profiles: service.profiles,
                }
            })
            .collect(),
        source,
    })
}

fn normalize_project_source(paths: Vec<String>) -> Result<ComposeProjectSource, RuntimeError> {
    if paths.is_empty() {
        return Err(RuntimeError::new("COMPOSE_SOURCE_EMPTY", false));
    }

    let selected_paths: Vec<_> = paths.into_iter().map(PathBuf::from).collect();
    if selected_paths.len() == 1 && selected_paths[0].is_dir() {
        return source_from_directory(&selected_paths[0]);
    }
    if selected_paths.iter().any(|path| !path.is_file()) {
        return Err(RuntimeError::new("COMPOSE_FILE_NOT_FOUND", false));
    }

    let compose_files = selected_paths
        .into_iter()
        .map(|path| canonicalize(&path))
        .collect::<Result<Vec<_>, _>>()?;
    let working_directory = compose_files
        .first()
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| RuntimeError::new("COMPOSE_SOURCE_INVALID", false))?;
    Ok(ComposeProjectSource {
        compose_files,
        working_directory,
    })
}

fn source_from_directory(directory: &Path) -> Result<ComposeProjectSource, RuntimeError> {
    let working_directory = canonicalize(directory)?;
    let base_file = [
        "compose.yaml",
        "compose.yml",
        "docker-compose.yaml",
        "docker-compose.yml",
    ]
    .into_iter()
    .map(|name| working_directory.join(name))
    .find(|path| path.is_file())
    .ok_or_else(|| RuntimeError::new("COMPOSE_FILE_NOT_FOUND", false))?;
    let mut compose_files = vec![base_file];
    if let Some(override_file) = [
        "compose.override.yaml",
        "compose.override.yml",
        "docker-compose.override.yaml",
        "docker-compose.override.yml",
    ]
    .into_iter()
    .map(|name| working_directory.join(name))
    .find(|path| path.is_file())
    {
        compose_files.push(override_file);
    }

    Ok(ComposeProjectSource {
        compose_files,
        working_directory,
    })
}

fn canonicalize(path: &Path) -> Result<PathBuf, RuntimeError> {
    fs::canonicalize(path).map_err(|_| RuntimeError::new("COMPOSE_FILE_NOT_FOUND", false))
}

fn default_configuration_path() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CONFIG_DIRECTORY_NAME)
        .join(CONFIG_FILE_NAME)
}

fn read_configuration(path: &Path) -> Result<CatalogConfiguration, RuntimeError> {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CatalogConfiguration::default());
        }
        Err(_) => return Err(RuntimeError::new("CONFIG_READ_FAILED", true)),
    };
    let configuration: CatalogConfiguration = serde_json::from_slice(&contents)
        .map_err(|_| RuntimeError::new("CONFIG_INVALID", false))?;
    if configuration.schema_version != CONFIG_SCHEMA_VERSION {
        return Err(RuntimeError::new("CONFIG_VERSION_UNSUPPORTED", false));
    }
    Ok(configuration)
}

fn write_configuration(
    path: &Path,
    configuration: &CatalogConfiguration,
) -> Result<(), RuntimeError> {
    let parent = path
        .parent()
        .ok_or_else(|| RuntimeError::new("CONFIG_WRITE_FAILED", false))?;
    fs::create_dir_all(parent).map_err(|_| RuntimeError::new("CONFIG_WRITE_FAILED", true))?;
    let temporary_path = path.with_extension("json.tmp");
    let contents = serde_json::to_vec_pretty(configuration)
        .map_err(|_| RuntimeError::new("CONFIG_WRITE_FAILED", false))?;
    fs::write(&temporary_path, contents)
        .map_err(|_| RuntimeError::new("CONFIG_WRITE_FAILED", true))?;
    fs::rename(&temporary_path, path).map_err(|_| RuntimeError::new("CONFIG_WRITE_FAILED", true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeComposeReader {
        definition: Mutex<Option<ComposeProjectDefinition>>,
    }

    impl ComposeProjectReader for FakeComposeReader {
        fn inspect<'a>(
            &'a self,
            _source: &'a ComposeProjectSource,
        ) -> BoxFuture<'a, Result<ComposeProjectDefinition, RuntimeError>> {
            Box::pin(async move {
                self.definition
                    .lock()
                    .expect("fake reader should not be poisoned")
                    .clone()
                    .ok_or_else(|| RuntimeError::new("MISSING_FAKE_DEFINITION", false))
            })
        }
    }

    fn source() -> ComposeProjectSource {
        ComposeProjectSource {
            compose_files: vec![PathBuf::from("/workspace/zerniki/compose.yaml")],
            working_directory: PathBuf::from("/workspace/zerniki"),
        }
    }

    fn definition() -> ComposeProjectDefinition {
        ComposeProjectDefinition {
            id: "compose:zerniki".to_owned(),
            name: "zerniki".to_owned(),
            services: vec![
                ComposeServiceDefinition {
                    name: "postgres".to_owned(),
                    profiles: Vec::new(),
                },
                ComposeServiceDefinition {
                    name: "postgres-test".to_owned(),
                    profiles: vec!["test".to_owned()],
                },
            ],
            source: source(),
        }
    }

    fn catalog() -> ProjectCatalog {
        let path = env::temp_dir().join(format!(
            "dockermon-catalog-test-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        let _ = fs::remove_file(&path);
        ProjectCatalog::load(
            path,
            Arc::new(FakeComposeReader {
                definition: Mutex::new(Some(definition())),
            }),
        )
    }

    #[test]
    fn parses_only_safe_compose_metadata() {
        let json = br#"{
          "name": "zerniki",
          "services": {
            "postgres": {"environment": {"PASSWORD": "secret"}},
            "postgres-test": {"profiles": ["test"], "environment": {"TOKEN": "secret"}}
          }
        }"#;

        let project = parse_compose_config(json, source()).expect("compose config should parse");

        assert_eq!(project.name, "zerniki");
        assert_eq!(project.services.len(), 2);
        assert_eq!(project.services[1].profiles, vec!["test"]);
        let serialized = serde_json::to_string(&StoredProject::from(project))
            .expect("stored project should serialize");
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("PASSWORD"));
        assert!(!serialized.contains("TOKEN"));
    }

    #[test]
    fn disabled_profiles_remain_visible_without_joining_the_project_scope() {
        let mut catalog = catalog();
        catalog.upsert_project(definition());

        let projects = catalog.decorate_projects(Vec::new());

        assert_eq!(projects[0].services.len(), 2);
        assert!(projects[0].services[0].included);
        assert!(!projects[0].services[1].included);
        assert_eq!(projects[0].profiles[0].name, "test");
        assert!(!projects[0].profiles[0].enabled);
    }

    #[test]
    fn enabling_a_profile_includes_and_selects_its_services_by_default() {
        let mut catalog = catalog();
        catalog.upsert_project(definition());
        catalog
            .set_project_profiles("compose:zerniki", vec!["test".to_owned()])
            .expect("profile selection should persist");

        let projects = catalog.decorate_projects(Vec::new());
        let test_service = projects[0]
            .services
            .iter()
            .find(|service| service.name == "postgres-test")
            .expect("test service should exist");

        assert!(test_service.included);
        assert!(test_service.bulk_selected);
        let _ = fs::remove_file(&catalog.configuration_path);
    }

    #[tokio::test]
    async fn automatically_registers_labeled_projects_and_enables_running_profiles() {
        let mut catalog = catalog();
        let projects = catalog
            .merge_projects(
                Vec::new(),
                vec![DiscoveredProject {
                    compose_files: source().compose_files,
                    id: "compose:zerniki".to_owned(),
                    name: "zerniki".to_owned(),
                    running_services: BTreeSet::from(["postgres-test".to_owned()]),
                    working_directory: source().working_directory,
                }],
            )
            .await;

        assert_eq!(projects.len(), 1);
        assert!(projects[0].profiles[0].enabled);
        assert!(projects[0].services[1].included);
        let _ = fs::remove_file(&catalog.configuration_path);
    }

    #[tokio::test]
    async fn removed_projects_stay_ignored_until_they_are_imported_again() {
        let mut catalog = catalog();
        catalog.upsert_project(definition());
        catalog
            .remove_project("compose:zerniki")
            .expect("project removal should persist");
        let configuration_path = catalog.configuration_path.clone();

        let mut reloaded = ProjectCatalog::load(
            configuration_path.clone(),
            Arc::new(FakeComposeReader {
                definition: Mutex::new(Some(definition())),
            }),
        );
        assert!(reloaded.decorate_projects(Vec::new()).is_empty());

        let compose_file = env::temp_dir().join(format!(
            "dockermon-reimport-test-{}.yaml",
            std::process::id()
        ));
        fs::write(&compose_file, "services: {}").expect("compose fixture should be written");
        reloaded
            .import_project(vec![compose_file.to_string_lossy().into_owned()])
            .await
            .expect("reimport should restore the project");

        assert_eq!(reloaded.decorate_projects(Vec::new()).len(), 1);
        fs::remove_file(compose_file).expect("compose fixture should be removed");
        let _ = fs::remove_file(configuration_path);
    }

    #[test]
    fn preserves_the_selected_compose_file_order() {
        let directory = env::temp_dir().join(format!(
            "dockermon-compose-order-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("test directory should be created");
        let override_file = directory.join("compose.override.yaml");
        let base_file = directory.join("compose.yaml");
        fs::write(&override_file, "services: {}").expect("override fixture should be written");
        fs::write(&base_file, "services: {}").expect("base fixture should be written");

        let source = normalize_project_source(vec![
            override_file.to_string_lossy().into_owned(),
            base_file.to_string_lossy().into_owned(),
        ])
        .expect("selected files should normalize");

        assert!(source.compose_files[0].ends_with("compose.override.yaml"));
        assert!(source.compose_files[1].ends_with("compose.yaml"));
        fs::remove_file(override_file).expect("override fixture should be removed");
        fs::remove_file(base_file).expect("base fixture should be removed");
        fs::remove_dir(directory).expect("test directory should be removed");
    }

    #[test]
    fn persists_preferences_activity_profiles_and_bulk_selection() {
        let mut catalog = catalog();
        catalog.upsert_project(definition());
        catalog
            .set_project_active("compose:zerniki", true)
            .expect("activity should persist");
        catalog
            .set_project_profiles("compose:zerniki", vec!["test".to_owned()])
            .expect("profiles should persist");
        catalog
            .set_bulk_selected("compose:zerniki", "postgres-test", false)
            .expect("bulk selection should persist");
        catalog
            .set_language(Locale::Pl)
            .expect("language should persist");
        catalog
            .set_theme(ThemePreference::Dark)
            .expect("theme should persist");
        let reloaded = ProjectCatalog::load(
            catalog.configuration_path.clone(),
            Arc::new(FakeComposeReader {
                definition: Mutex::new(Some(definition())),
            }),
        );

        let projects = reloaded.decorate_projects(Vec::new());

        assert!(projects[0].active);
        assert!(projects[0].profiles[0].enabled);
        assert!(!projects[0].services[1].bulk_selected);
        assert_eq!(reloaded.preferences().language, Locale::Pl);
        assert_eq!(reloaded.preferences().theme, ThemePreference::Dark);
        let _ = fs::remove_file(&catalog.configuration_path);
    }

    #[test]
    fn exposes_only_safe_compose_execution_metadata() {
        let mut catalog = catalog();
        catalog.upsert_project(definition());
        catalog
            .set_project_profiles("compose:zerniki", vec!["test".to_owned()])
            .expect("profiles should persist");

        let context = catalog
            .execution_context("compose:zerniki")
            .expect("stored project should expose its execution context");

        assert_eq!(context.compose_files, source().compose_files);
        assert_eq!(context.working_directory, source().working_directory);
        assert_eq!(context.enabled_profiles, vec!["test"]);
        let _ = fs::remove_file(&catalog.configuration_path);
    }
}
