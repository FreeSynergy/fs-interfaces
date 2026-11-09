//! The `TypeCatalog` server answers over a real Cap'n Proto client.
#![cfg(feature = "rpc")]

use fs_interfaces::types_capnp::type_catalog;
use fs_types::describe::CatalogServer;
use fs_types::{BasicType, FieldDescription, TypeDescription, TypeRegistry};

#[tokio::test(flavor = "current_thread")]
async fn describe_answers_the_registry() {
    let mut reg = TypeRegistry::new();
    reg.add(
        TypeDescription::new("Person", "fs-people", "RFC 9553 Card", "person").field(
            FieldDescription::new("emails", BasicType::Email)
                .multiple()
                .label("person.emails"),
        ),
    )
    .unwrap();
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async move {
            let client: type_catalog::Client =
                capnp_rpc::new_client(CatalogServer::new(reg.clone()));
            let response = client.describe_request().send().promise.await.unwrap();
            let back = TypeRegistry::read(response.get().unwrap().get_types().unwrap()).unwrap();
            assert_eq!(back.get("Person"), reg.get("Person"));
        })
        .await;
}
