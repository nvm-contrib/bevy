use crate::{DynamicScene, Scene};
use bevy_asset::{AssetEvent, AssetId, Assets};
use bevy_ecs::{
    entity::Entity,
    event::{Event, Events, ManualEventReader},
    reflect::AppTypeRegistry,
    system::{Command, Resource},
    world::{Mut, World},
};
use bevy_hierarchy::{AddChild, Parent};
use bevy_utils::{tracing::error, HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

/// Emitted when [`crate::SceneInstance`] becomes ready to use.
///
/// See also [`SceneSpawner::instance_is_ready`].
#[derive(Event)]
pub struct SceneInstanceReady {
    /// Entity to which the scene was spawned as a child.
    pub parent: Entity,
}

/// Information about a scene instance.
#[derive(Debug)]
pub struct InstanceInfo {
    /// Mapping of entities from the scene world to the instance world.
    pub entity_map: HashMap<Entity, Entity>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct InstanceId(Uuid);

impl InstanceId {
    fn new() -> Self {
        InstanceId(Uuid::new_v4())
    }
}

#[derive(Default, Resource)]
pub struct SceneSpawner {
    spawned_scenes: HashMap<AssetId<Scene>, Vec<InstanceId>>,
    spawned_dynamic_scenes: HashMap<AssetId<DynamicScene>, Vec<InstanceId>>,
    spawned_instances: HashMap<InstanceId, InstanceInfo>,
    scene_asset_event_reader: ManualEventReader<AssetEvent<DynamicScene>>,
    dynamic_scenes_to_spawn: Vec<(AssetId<DynamicScene>, InstanceId)>,
    scenes_to_spawn: Vec<(AssetId<Scene>, InstanceId)>,
    scenes_to_despawn: Vec<AssetId<DynamicScene>>,
    instances_to_despawn: Vec<InstanceId>,
    scenes_with_parent: Vec<(InstanceId, Entity)>,
}

#[derive(Error, Debug)]
pub enum SceneSpawnError {
    #[error("scene contains the unregistered component `{type_name}`. consider adding `#[reflect(Component)]` to your type")]
    UnregisteredComponent { type_name: String },
    #[error("scene contains the unregistered resource `{type_name}`. consider adding `#[reflect(Resource)]` to your type")]
    UnregisteredResource { type_name: String },
    #[error("scene contains the unregistered type `{type_name}`. consider registering the type using `app.register_type::<T>()`")]
    UnregisteredType { type_name: String },
    #[error("scene does not exist")]
    NonExistentScene { id: AssetId<DynamicScene> },
    #[error("scene does not exist")]
    NonExistentRealScene { id: AssetId<Scene> },
}

impl SceneSpawner {
    pub fn spawn_dynamic(&mut self, id: impl Into<AssetId<DynamicScene>>) -> InstanceId {
        let instance_id = InstanceId::new();
        self.dynamic_scenes_to_spawn.push((id.into(), instance_id));
        instance_id
    }

    pub fn spawn_dynamic_as_child(
        &mut self,
        id: impl Into<AssetId<DynamicScene>>,
        parent: Entity,
    ) -> InstanceId {
        let instance_id = InstanceId::new();
        self.dynamic_scenes_to_spawn.push((id.into(), instance_id));
        self.scenes_with_parent.push((instance_id, parent));
        instance_id
    }

    pub fn spawn(&mut self, id: impl Into<AssetId<Scene>>) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn.push((id.into(), instance_id));
        instance_id
    }

    pub fn spawn_as_child(&mut self, id: impl Into<AssetId<Scene>>, parent: Entity) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn.push((id.into(), instance_id));
        self.scenes_with_parent.push((instance_id, parent));
        instance_id
    }

    pub fn despawn(&mut self, id: impl Into<AssetId<DynamicScene>>) {
        self.scenes_to_despawn.push(id.into());
    }

    pub fn despawn_instance(&mut self, instance_id: InstanceId) {
        self.instances_to_despawn.push(instance_id);
    }

    pub fn despawn_sync(
        &mut self,
        world: &mut World,
        id: impl Into<AssetId<DynamicScene>>,
    ) -> Result<(), SceneSpawnError> {
        if let Some(instance_ids) = self.spawned_dynamic_scenes.remove(&id.into()) {
            for instance_id in instance_ids {
                self.despawn_instance_sync(world, &instance_id);
            }
        }
        Ok(())
    }

    pub fn despawn_instance_sync(&mut self, world: &mut World, instance_id: &InstanceId) {
        if let Some(instance) = self.spawned_instances.remove(instance_id) {
            for &entity in instance.entity_map.values() {
                let _ = world.despawn(entity);
            }
        }
    }

    pub fn spawn_dynamic_sync(
        &mut self,
        world: &mut World,
        id: impl Into<AssetId<DynamicScene>>,
    ) -> Result<(), SceneSpawnError> {
        let mut entity_map = HashMap::default();
        let id = id.into();
        Self::spawn_dynamic_internal(world, id, &mut entity_map)?;
        let instance_id = InstanceId::new();
        self.spawned_instances
            .insert(instance_id, InstanceInfo { entity_map });
        let spawned = self.spawned_dynamic_scenes.entry(id).or_default();
        spawned.push(instance_id);
        Ok(())
    }

    fn spawn_dynamic_internal(
        world: &mut World,
        id: AssetId<DynamicScene>,
        entity_map: &mut HashMap<Entity, Entity>,
    ) -> Result<(), SceneSpawnError> {
        world.resource_scope(|world, scenes: Mut<Assets<DynamicScene>>| {
            let scene = scenes
                .get(id)
                .ok_or(SceneSpawnError::NonExistentScene { id })?;
            scene.write_to_world(world, entity_map)
        })
    }

    pub fn spawn_sync(
        &mut self,
        world: &mut World,
        id: AssetId<Scene>,
    ) -> Result<InstanceId, SceneSpawnError> {
        self.spawn_sync_internal(world, id, InstanceId::new())
    }

    fn spawn_sync_internal(
        &mut self,
        world: &mut World,
        id: AssetId<Scene>,
        instance_id: InstanceId,
    ) -> Result<InstanceId, SceneSpawnError> {
        world.resource_scope(|world, scenes: Mut<Assets<Scene>>| {
            let scene = scenes
                .get(id)
                .ok_or(SceneSpawnError::NonExistentRealScene { id })?;

            let instance_info =
                scene.write_to_world_with(world, &world.resource::<AppTypeRegistry>().clone())?;

            self.spawned_instances.insert(instance_id, instance_info);
            let spawned = self.spawned_scenes.entry(id).or_default();
            spawned.push(instance_id);
            Ok(instance_id)
        })
    }

    pub fn update_spawned_scenes(
        &mut self,
        world: &mut World,
        scene_ids: &[AssetId<DynamicScene>],
    ) -> Result<(), SceneSpawnError> {
        for id in scene_ids {
            if let Some(spawned_instances) = self.spawned_dynamic_scenes.get(id) {
                for instance_id in spawned_instances {
                    if let Some(instance_info) = self.spawned_instances.get_mut(instance_id) {
                        Self::spawn_dynamic_internal(world, *id, &mut instance_info.entity_map)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn despawn_queued_scenes(&mut self, world: &mut World) -> Result<(), SceneSpawnError> {
        let scenes_to_despawn = std::mem::take(&mut self.scenes_to_despawn);

        for scene_handle in scenes_to_despawn {
            self.despawn_sync(world, scene_handle)?;
        }
        Ok(())
    }

    pub fn despawn_queued_instances(&mut self, world: &mut World) {
        let instances_to_despawn = std::mem::take(&mut self.instances_to_despawn);

        for instance_id in instances_to_despawn {
            self.despawn_instance_sync(world, &instance_id);
        }
    }

    pub fn spawn_queued_scenes(&mut self, world: &mut World) -> Result<(), SceneSpawnError> {
        let scenes_to_spawn = std::mem::take(&mut self.dynamic_scenes_to_spawn);

        for (id, instance_id) in scenes_to_spawn {
            let mut entity_map = HashMap::default();

            match Self::spawn_dynamic_internal(world, id, &mut entity_map) {
                Ok(_) => {
                    self.spawned_instances
                        .insert(instance_id, InstanceInfo { entity_map });
                    let spawned = self
                        .spawned_dynamic_scenes
                        .entry(id)
                        .or_insert_with(Vec::new);
                    spawned.push(instance_id);
                }
                Err(SceneSpawnError::NonExistentScene { .. }) => {
                    self.dynamic_scenes_to_spawn.push((id, instance_id));
                }
                Err(err) => return Err(err),
            }
        }

        let scenes_to_spawn = std::mem::take(&mut self.scenes_to_spawn);

        for (scene_handle, instance_id) in scenes_to_spawn {
            match self.spawn_sync_internal(world, scene_handle, instance_id) {
                Ok(_) => {}
                Err(SceneSpawnError::NonExistentRealScene { id: handle }) => {
                    self.scenes_to_spawn.push((handle, instance_id));
                }
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }

    pub(crate) fn set_scene_instance_parent_sync(&mut self, world: &mut World) {
        let scenes_with_parent = std::mem::take(&mut self.scenes_with_parent);

        for (instance_id, parent) in scenes_with_parent {
            if let Some(instance) = self.spawned_instances.get(&instance_id) {
                for &entity in instance.entity_map.values() {
                    // Add the `Parent` component to the scene root, and update the `Children` component of
                    // the scene parent
                    if !world
                        .get_entity(entity)
                        // This will filter only the scene root entity, as all other from the
                        // scene have a parent
                        .map(|entity| entity.contains::<Parent>())
                        // Default is true so that it won't run on an entity that wouldn't exist anymore
                        // this case shouldn't happen anyway
                        .unwrap_or(true)
                    {
                        AddChild {
                            parent,
                            child: entity,
                        }
                        .apply(world);

                        world.send_event(SceneInstanceReady { parent });
                    }
                }
            } else {
                self.scenes_with_parent.push((instance_id, parent));
            }
        }
    }

    /// Check that an scene instance spawned previously is ready to use
    pub fn instance_is_ready(&self, instance_id: InstanceId) -> bool {
        self.spawned_instances.contains_key(&instance_id)
    }

    /// Get an iterator over the entities in an instance, once it's spawned.
    ///
    /// Before the scene is spawned, the iterator will be empty. Use [`Self::instance_is_ready`]
    /// to check if the instance is ready.
    pub fn iter_instance_entities(
        &'_ self,
        instance_id: InstanceId,
    ) -> impl Iterator<Item = Entity> + '_ {
        self.spawned_instances
            .get(&instance_id)
            .map(|instance| instance.entity_map.values())
            .into_iter()
            .flatten()
            .copied()
    }
}

pub fn scene_spawner_system(world: &mut World) {
    world.resource_scope(|world, mut scene_spawner: Mut<SceneSpawner>| {
        // remove any loading instances where parent is deleted
        let mut dead_instances = HashSet::default();
        scene_spawner
            .scenes_with_parent
            .retain(|(instance, parent)| {
                let retain = world.get_entity(*parent).is_some();

                if !retain {
                    dead_instances.insert(*instance);
                }

                retain
            });
        scene_spawner
            .dynamic_scenes_to_spawn
            .retain(|(_, instance)| !dead_instances.contains(instance));
        scene_spawner
            .scenes_to_spawn
            .retain(|(_, instance)| !dead_instances.contains(instance));

        let scene_asset_events = world.resource::<Events<AssetEvent<DynamicScene>>>();

        let mut updated_spawned_scenes = Vec::new();
        let scene_spawner = &mut *scene_spawner;
        for event in scene_spawner
            .scene_asset_event_reader
            .read(scene_asset_events)
        {
            if let AssetEvent::Modified { id } = event {
                if scene_spawner.spawned_dynamic_scenes.contains_key(id) {
                    updated_spawned_scenes.push(*id);
                }
            }
        }

        scene_spawner.despawn_queued_scenes(world).unwrap();
        scene_spawner.despawn_queued_instances(world);
        scene_spawner
            .spawn_queued_scenes(world)
            .unwrap_or_else(|err| panic!("{}", err));
        scene_spawner
            .update_spawned_scenes(world, &updated_spawned_scenes)
            .unwrap();
        scene_spawner.set_scene_instance_parent_sync(world);
    });
}
