use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn objectid_new() {
    let _ = bson::oid::ObjectId::new();
}