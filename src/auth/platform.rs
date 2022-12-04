use oso::PolarClass;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Platform {
    id: Uuid,
}

impl Platform {
    pub fn default() -> Self {
        Self { id: Uuid::nil() }
    }
}

impl PolarClass for Platform {
    fn get_polar_class_builder() -> oso::ClassBuilder<Platform> {
        oso::Class::builder()
            .name("Platform")
            .add_attribute_getter("id", |recv: &Platform| recv.id.clone())
            .add_class_method("default", Platform::default)
    }

    fn get_polar_class() -> oso::Class {
        let builder = Platform::get_polar_class_builder();
        builder.build()
    }
}
