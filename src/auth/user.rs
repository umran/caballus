use oso::PolarClass;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub roles: Vec<String>,
}

impl User {
    pub fn new_system_user() -> Self {
        Self {
            id: Uuid::new_v4(),
            roles: vec!["system".into()],
        }
    }

    fn id_equals_nullable_id(&self, optional_id: Option<Uuid>) -> bool {
        if let Some(id) = optional_id {
            if self.id == id {
                return true;
            }
        }

        false
    }

    fn has_role(&self, role: String) -> bool {
        self.roles.iter().find(|&x| x == &role).is_some()
    }
}

impl PolarClass for User {
    fn get_polar_class_builder() -> oso::ClassBuilder<User> {
        oso::Class::builder()
            .name("User")
            .add_attribute_getter("id", |recv: &User| recv.id.clone())
            .add_attribute_getter("roles", |recv: &User| recv.roles.clone())
            .add_method("id_equals_nullable_id", User::id_equals_nullable_id)
            .add_method("has_role", User::has_role)
    }

    fn get_polar_class() -> oso::Class {
        let builder = User::get_polar_class_builder();
        builder.build()
    }
}
