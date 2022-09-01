use arangodb_types::models::type_model;

type_model!(
    #![build_api]

    pub struct TestStruct {
        #[db_name = "_key"]
        pub db_key: Option<u64>,

        #[db_name = "V"]
        pub value: NullableOption<u64>,
    }
);

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

type_model!(
    #![build_api]

    pub enum TestSimpleEnum {
        #[db_name = "D"]
        #[inner_model = "data"]
        Data,

        #[db_name = "S"]
        SubObject,
    }
);

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

type_model!(
    #![build_api]

    pub enum TestComplexEnum {
        #[db_name = "D"]
        #[inner_model = "data"]
        Data(u64),

        #[db_name = "S"]
        SubObject(TestStruct),

        #[db_name = "E"]
        #[inner_model = "enum"]
        SubEnum(TestSimpleEnum),
    }
);

#[test]
fn x() {
    let x = serde_json::to_string(&ApiTestSimpleEnum::Data).unwrap();
    println!("{}", x);

    let x = serde_json::to_string(&ApiTestComplexEnum::Data(54)).unwrap();
    println!("{}", x)
}
