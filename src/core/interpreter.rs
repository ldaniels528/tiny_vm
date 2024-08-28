////////////////////////////////////////////////////////////////////
// interpreter module
////////////////////////////////////////////////////////////////////

use serde::{Deserialize, Serialize};

use crate::compiler::Compiler;
use crate::machine::Machine;
use crate::typed_values::TypedValue;

/// Represents the Oxide language interpreter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Interpreter {
    machine: Machine,
}

impl Interpreter {

    ////////////////////////////////////////////////////////////////
    // static methods
    ////////////////////////////////////////////////////////////////

    /// Constructs a new Interpreter
    pub fn new() -> Self {
        Interpreter {
            machine: Machine::new(),
        }
    }

    ////////////////////////////////////////////////////////////////
    // instance methods
    ////////////////////////////////////////////////////////////////

    /// Executes the supplied source code returning the result of the evaluation
    pub(crate) fn evaluate(&mut self, source_code: &str) -> std::io::Result<TypedValue> {
        let opcode = Compiler::compile_script(source_code)?;
        let (machine, result) = self.machine.evaluate(&opcode)?;
        self.machine = machine;
        Ok(result)
    }

    /// Executes the supplied source code returning the result of the evaluation
    pub async fn evaluate_async(&mut self, source_code: &str) -> std::io::Result<TypedValue> {
        let opcode = Compiler::compile_script(source_code)?;
        let (machine, result) = self.machine.evaluate_async(&opcode).await?;
        self.machine = machine;
        Ok(result)
    }

    pub fn with_variable(&mut self, name: &str, value: TypedValue) {
        self.machine = self.machine.with_variable(name, value);
    }
}

// Unit tests
#[cfg(test)]
mod tests {
    use regex::Regex;
    use serde_json::json;

    use shared_lib::{FieldJs, RowJs};

    use crate::interpreter::Interpreter;
    use crate::model_row_collection::ModelRowCollection;
    use crate::rows::Row;
    use crate::structure::Structure;
    use crate::table_columns::TableColumn;
    use crate::table_renderer::TableRenderer;
    use crate::testdata::{make_quote, make_quote_columns, make_scan_quote, make_table_columns};
    use crate::typed_values::TypedValue;
    use crate::typed_values::TypedValue::{Ack, Array, Boolean, ErrorValue, Float64Value, Int64Value, JSONObjectValue, Null, RowsAffected, StringValue, StructureValue, TableValue, Undefined};

    #[actix::test]
    async fn test_array_indexing() {
        let mut interpreter = Interpreter::new();
        assert_eq!(interpreter.evaluate("[0, 1, 3, 5][3]").unwrap(), Int64Value(5))
    }

    #[actix::test]
    async fn test_basic_state_retention() {
        let mut interpreter = Interpreter::new();
        assert_eq!(interpreter.evaluate("x := 5").unwrap(), TypedValue::Ack);
        assert_eq!(interpreter.evaluate("x¡").unwrap(), TypedValue::Float64Value(120.));
        assert_eq!(interpreter.evaluate("x := x + 1").unwrap(), TypedValue::Ack);
        assert_eq!(interpreter.evaluate("x").unwrap(), TypedValue::Int64Value(6));
        assert_eq!(interpreter.evaluate("x < 7").unwrap(), TypedValue::Boolean(true));
        assert_eq!(interpreter.evaluate("x := x ** 2").unwrap(), TypedValue::Ack);
        assert_eq!(interpreter.evaluate("x").unwrap(), TypedValue::Int64Value(36));
        assert_eq!(interpreter.evaluate("x / 0").unwrap(), TypedValue::Int64Value(9223372036854775807));
        assert_eq!(interpreter.evaluate("x := x - 1").unwrap(), TypedValue::Ack);
        assert_eq!(interpreter.evaluate("x % 5").unwrap(), TypedValue::Int64Value(0));
        assert_eq!(interpreter.evaluate("x < 35").unwrap(), TypedValue::Boolean(false));
        assert_eq!(interpreter.evaluate("x >= 35").unwrap(), TypedValue::Boolean(true));
    }

    #[actix::test]
    async fn test_compact_from_namespace() {
        let phys_columns = make_table_columns();
        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(r#"
            [+] stocks := ns("interpreter.compact.stocks")
            [+] table(symbol: String(8), exchange: String(8), last_sale: f64) ~> stocks
            [+] [{ symbol: "DMX", exchange: "NYSE", last_sale: 99.99 },
                 { symbol: "UNO", exchange: "OTC", last_sale: 0.2456 },
                 { symbol: "BIZ", exchange: "NYSE", last_sale: 23.66 },
                 { symbol: "GOTO", exchange: "OTC", last_sale: 0.1428 },
                 { symbol: "ABC", exchange: "AMEX", last_sale: 11.11 },
                 { symbol: "BOOM", exchange: "NASDAQ", last_sale: 0.0872 },
                 { symbol: "JET", exchange: "NASDAQ", last_sale: 32.12 }] ~> stocks
            [+] delete from stocks where last_sale > 1.0
            [+] from stocks
        "#).unwrap();

        let rc = result.to_table().unwrap();
        assert_eq!(rc.read_active_rows().unwrap(), vec![
            make_quote(1, &phys_columns, "UNO", "OTC", 0.2456),
            make_quote(3, &phys_columns, "GOTO", "OTC", 0.1428),
            make_quote(5, &phys_columns, "BOOM", "NASDAQ", 0.0872),
        ]);

        let result = interpreter.evaluate(r#"
            [+] compact stocks
            [+] from stocks
        "#).unwrap();
        let rc = result.to_table().unwrap();
        let rows = rc.read_active_rows().unwrap();
        for s in TableRenderer::from_rows(rows.clone()) { println!("{}", s); }
        assert_eq!(rows, vec![
            make_quote(0, &phys_columns, "BOOM", "NASDAQ", 0.0872),
            make_quote(1, &phys_columns, "UNO", "OTC", 0.2456),
            make_quote(2, &phys_columns, "GOTO", "OTC", 0.1428),
        ]);
    }

    #[actix::test]
    async fn test_create_structure_from_scratch() {
        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(r#"
            stock := struct(symbol: String(8), exchange: String(8), last_sale: f64)
            stock
        "#).unwrap();
        assert_eq!(result, StructureValue(Structure::from_logical_columns(&make_quote_columns()).unwrap()));
    }

    #[actix::test]
    async fn test_create_structure_from_table() {
        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(r#"
            [+] stocks := ns("interpreter.struct.stocks")
            [+] table(symbol: String(8), exchange: String(8), last_sale: f64) ~> stocks
            [+] [{ symbol: "ABC", exchange: "AMEX", last_sale: 11.11 },
                 { symbol: "UNO", exchange: "OTC", last_sale: 0.2456 },
                 { symbol: "BIZ", exchange: "NYSE", last_sale: 23.66 },
                 { symbol: "GOTO", exchange: "OTC", last_sale: 0.1428 },
                 { symbol: "BOOM", exchange: "NASDAQ", last_sale: 0.0872 }] ~> stocks
            stocks[0]
        "#).unwrap();
        assert_eq!(result, StructureValue(Structure::from_logical_columns_and_values(
            &make_quote_columns(), vec![
                StringValue("ABC".to_string()),
                StringValue("AMEX".to_string()),
                Float64Value(11.11),
            ],
        ).unwrap()));
    }

    #[actix::test]
    async fn test_table_to_csv() {
        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(r#"
            [+] stocks := ns("interpreter.csv.stocks")
            [+] table(symbol: String(8), exchange: String(8), last_sale: f64) ~> stocks
            [+] [{ symbol: "ABC", exchange: "AMEX", last_sale: 11.11 },
                 { symbol: "UNO", exchange: "OTC", last_sale: 0.2456 },
                 { symbol: "BIZ", exchange: "NYSE", last_sale: 23.66 },
                 { symbol: "GOTO", exchange: "OTC", last_sale: 0.1428 },
                 { symbol: "BOOM", exchange: "NASDAQ", last_sale: 0.0872 }] ~> stocks
            csv(from stocks)
        "#).unwrap();
        assert_eq!(result, TypedValue::Array(vec![
            StringValue(r#""ABC","AMEX",11.11"#.to_string()),
            StringValue(r#""UNO","OTC",0.2456"#.to_string()),
            StringValue(r#""BIZ","NYSE",23.66"#.to_string()),
            StringValue(r#""GOTO","OTC",0.1428"#.to_string()),
            StringValue(r#""BOOM","NASDAQ",0.0872"#.to_string()),
        ]));
    }

    #[actix::test]
    async fn test_describe_table_structure() {
        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(r#"
            [+] stocks := ns("interpreter.struct.stocks")
            [+] table(symbol: String(8), exchange: String(8), last_sale: f64) ~> stocks
            describe stocks
        "#).unwrap();

        // process the table result
        // |-----------------------------------------------------|
        // | name      | type      | default_value | is_nullable |
        // |-----------------------------------------------------|
        // | symbol    | String(8) | null          | true        |
        // | exchange  | String(8) | null          | true        |
        // | last_sale | f64       | null          | true        |
        // |-----------------------------------------------------|
        let mrc = result.to_table().unwrap();
        let mrc_rows = mrc.read_active_rows().unwrap();
        let mrc_columns = mrc.get_columns();
        for s in TableRenderer::from_rows(mrc_rows.clone()) { println!("{}", s); }
        assert_eq!(mrc_rows, vec![
            Row::new(0, mrc_columns.clone(), vec![
                StringValue("symbol".to_string()),
                StringValue("String(8)".to_string()),
                StringValue("null".to_string()),
                Boolean(true),
            ]),
            Row::new(1, mrc_columns.clone(), vec![
                StringValue("exchange".to_string()),
                StringValue("String(8)".to_string()),
                StringValue("null".to_string()),
                Boolean(true),
            ]),
            Row::new(2, mrc_columns.clone(), vec![
                StringValue("last_sale".to_string()),
                StringValue("f64".to_string()),
                StringValue("null".to_string()),
                Boolean(true),
            ]),
        ]);
    }

    #[actix::test]
    async fn test_directive_must_be_true() {
        verify_results("[+] x := 67", TypedValue::Ack);
    }

    #[actix::test]
    async fn test_directive_must_be_false() {
        verify_results(r#"
            [+] x := 67
            [-] x < 67
        "#, TypedValue::Boolean(false));
    }

    #[actix::test]
    async fn test_directive_die() {
        verify_results(r#"
            [!] "Kaboom!!!"
        "#, TypedValue::ErrorValue("Kaboom!!!".to_string()));
    }

    #[actix::test]
    async fn test_eval_valid() {
        verify_results(r#"
            eval "2 ** 4"
        "#, TypedValue::Int64Value(16))
    }

    #[actix::test]
    async fn test_eval_invalid() {
        let mut interpreter = Interpreter::new();
        let value = interpreter.evaluate(r#"
            eval 123
        "#).unwrap();
        assert_eq!(value, TypedValue::ErrorValue("Type mismatch - expected String, got i64".into()))
    }

    #[actix::test]
    async fn test_http_workflow() {
        let number_regex = Regex::new(r"^\d+$").unwrap();
        let mut interpreter = Interpreter::new();

        // setup a listener on port 8833
        let _ = interpreter.evaluate_async(r#"
            SERVE 8833
        "#).await.unwrap();

        // append a new row
        let row_id = interpreter.evaluate_async(r#"
            POST "http://localhost:8833/machine/www/stocks/0" FROM {
                fields:[
                    { name: "symbol", value: "ABC" },
                    { name: "exchange", value: "AMEX" },
                    { name: "last_sale", value: 11.77 }
                ]
            }"#).await.unwrap();
        println!("POST (row_id) {:?}", row_id);
        assert!(number_regex.is_match(row_id.unwrap_value().as_str()));

        // fetch the previously created row
        let row = interpreter.evaluate_async(format!(r#"
            GET "http://localhost:8833/machine/www/stocks/{row_id}"
        "#).as_str()).await.unwrap();
        println!("GET {:?}", row);
        let row_js: RowJs = serde_json::from_str(row.unwrap_value().as_str()).unwrap();
        assert_eq!(
            row_js,
            RowJs::new(row_js.get_id(), vec![
                FieldJs::new("symbol", json!("ABC")),
                FieldJs::new("exchange", json!("AMEX")),
                FieldJs::new("last_sale", json!(11.77)),
            ])
        );

        // replace the previously created row
        let result = interpreter.evaluate_async(r#"
            PUT "http://localhost:8833/machine/www/stocks/:id" FROM {
                fields:[
                    { name: "symbol", value: "ABC" },
                    { name: "exchange", value: "AMEX" },
                    { name: "last_sale", value: 11.79 }
                ]
            }"#
            .replace("/:id", format!("/{}", row_id.unwrap_value()).as_str())
            .as_str()).await.unwrap();
        println!("PUT {:?}", result);
        assert!(number_regex.is_match(row_id.unwrap_value().as_str()));

        // re-fetch the previously updated row
        let row = interpreter.evaluate_async(format!(r#"
            GET "http://localhost:8833/machine/www/stocks/{row_id}"
        "#).as_str()).await.unwrap();
        println!("GET {:?}", row);
        let row_js: RowJs = serde_json::from_str(row.unwrap_value().as_str()).unwrap();
        assert_eq!(
            row_js,
            RowJs::new(row_js.get_id(), vec![
                FieldJs::new("symbol", json!("ABC")),
                FieldJs::new("exchange", json!("AMEX")),
                FieldJs::new("last_sale", json!(11.79)),
            ])
        );

        // update the previously created row
        let result = interpreter.evaluate_async(r#"
            PATCH "http://localhost:8833/machine/www/stocks/:id" FROM {
                fields:[
                    { name: "last_sale", value: 11.81 }
                ]
            }"#
            .replace("/:id", format!("/{}", row_id.unwrap_value()).as_str())
            .as_str()).await.unwrap();
        println!("PATCH {:?}", result);
        assert!(number_regex.is_match(row_id.unwrap_value().as_str()));

        // re-fetch the previously updated row
        let row = interpreter.evaluate_async(format!(r#"
            GET "http://localhost:8833/machine/www/stocks/{row_id}"
        "#).as_str()).await.unwrap();
        println!("GET {:?}", row);
        let row_js: RowJs = serde_json::from_str(row.unwrap_value().as_str()).unwrap();
        assert_eq!(
            row_js,
            RowJs::new(row_js.get_id(), vec![
                FieldJs::new("symbol", json!("ABC")),
                FieldJs::new("exchange", json!("AMEX")),
                FieldJs::new("last_sale", json!(11.81)),
            ])
        );

        // fetch the headers for the previously updated row
        let result = interpreter.evaluate_async(format!(r#"
            HEAD "http://localhost:8833/machine/www/stocks/{row_id}"
        "#).as_str()).await.unwrap();
        println!("HEAD {:?}", result);

        // delete the previously updated row
        let result = interpreter.evaluate_async(format!(r#"
            DELETE "http://localhost:8833/machine/www/stocks/{row_id}"
        "#).as_str()).await.unwrap();
        println!("DELETE {:?}", result);
        assert_eq!(result.unwrap_value().as_str(), r#"1"#);

        // verify the deleted row is empty
        let row = interpreter.evaluate_async(format!(r#"
            GET "http://localhost:8833/machine/www/stocks/{row_id}"
        "#).as_str()).await.unwrap();
        assert_eq!(row.unwrap_value().as_str(), r#"{}"#);
        println!("GET {:?}", row);
    }

    #[actix::test]
    async fn test_if_when_result_is_defined() {
        let mut interpreter = Interpreter::new();
        let value = interpreter.evaluate(r#"
            x := 7
            if(x > 5) "Yes"
        "#).unwrap();
        assert_eq!(value, StringValue("Yes".to_string()));
    }

    #[actix::test]
    async fn test_if_when_result_is_undefined() {
        let mut interpreter = Interpreter::new();
        let value = interpreter.evaluate(r#"
            x := 4
            if(x > 5) "Yes"
        "#).unwrap();
        assert_eq!(value, Undefined);
    }

    #[actix::test]
    async fn test_if_else_expression() {
        let mut interpreter = Interpreter::new();
        let value = interpreter.evaluate(r#"
            x := 4
            if(x > 5) "Yes"
            else if(x < 5) "Maybe"
            else "No"
        "#).unwrap();
        assert_eq!(value, StringValue("Maybe".to_string()));
    }

    #[actix::test]
    async fn test_if_function() {
        verify_results(r#"
            x := 4
            iff(x > 5, "Yes", iff(x < 5, "Maybe", "No"))
        "#, StringValue("Maybe".to_string()));
    }

    #[actix::test]
    async fn test_include_file_valid() {
        let phys_columns = make_table_columns();
        verify_results(r#"
            include "./demoes/language/include_file.ox"
        "#, TableValue(ModelRowCollection::from_rows(vec![
            make_quote(0, &phys_columns, "ABC", "AMEX", 12.49),
            make_quote(1, &phys_columns, "BOOM", "NYSE", 56.88),
            make_quote(2, &phys_columns, "JET", "NASDAQ", 32.12),
        ])));
    }

    #[actix::test]
    async fn test_include_file_invalid() {
        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(r#"
        include 123
        "#).unwrap();
        assert_eq!(
            result, ErrorValue("Type mismatch - expected String, got i64".to_string())
        )
    }

    #[actix::test]
    async fn test_json_literal() {
        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(r#"
            {
              columns: [{
                  name: "symbol",
                  column_type: "String(8)",
                  default_value: null
                }, {
                  name: "exchange",
                  column_type: "String(8)",
                  default_value: null
                }, {
                  name: "last_sale",
                  column_type: "f64",
                  default_value: null
                }],
              indices: [],
              partitions: []
            }
        "#).unwrap();
        assert_eq!(result, JSONObjectValue(vec![
            ("columns".to_string(), Array(vec![
                JSONObjectValue(vec![
                    ("name".to_string(), StringValue("symbol".to_string())),
                    ("column_type".to_string(), StringValue("String(8)".to_string())),
                    ("default_value".to_string(), Null),
                ]),
                JSONObjectValue(vec![
                    ("name".to_string(), StringValue("exchange".to_string())),
                    ("column_type".to_string(), StringValue("String(8)".to_string())),
                    ("default_value".to_string(), Null)]),
                JSONObjectValue(vec![
                    ("name".to_string(), StringValue("last_sale".to_string())),
                    ("column_type".to_string(), StringValue("f64".to_string())),
                    ("default_value".to_string(), Null),
                ]),
            ])),
            ("indices".to_string(), Array(vec![])),
            ("partitions".to_string(), Array(vec![])),
        ]));
    }

    #[actix::test]
    async fn test_lambda_functions() {
        let mut interpreter = Interpreter::new();
        assert_eq!(Ack, interpreter.evaluate(r#"
        product := fn (a, b) => a * b
        "#).unwrap());

        assert_eq!(Int64Value(10), interpreter.evaluate(r#"
        product(2, 5)
        "#).unwrap())
    }

    #[actix::test]
    async fn test_named_functions() {
        let mut interpreter = Interpreter::new();
        assert_eq!(Ack, interpreter.evaluate(r#"
        fn product(a, b) => a * b
        "#).unwrap());

        assert_eq!(Int64Value(10), interpreter.evaluate(r#"
        product(2, 5)
        "#).unwrap())
    }

    #[actix::test]
    async fn test_create_ephemeral_table() {
        verify_results(r#"
        table(
            symbol: String(8),
            exchange: String(8),
            last_sale: f64
        )"#, TableValue(ModelRowCollection::with_rows(
            make_table_columns(), Vec::new(),
        )))
    }

    #[actix::test]
    async fn test_create_durable_table() {
        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(r#"
        create table ns("interpreter.create.stocks") (
            symbol: String(8),
            exchange: String(8),
            last_sale: f64
        )"#).unwrap();
        assert_eq!(result, Ack)
    }

    #[actix::test]
    async fn test_crud_in_namespace() {
        let mut interpreter = Interpreter::new();
        let columns = make_quote_columns();
        let phys_columns = TableColumn::from_columns(&columns).unwrap();

        // set up the interpreter
        assert_eq!(Ack, interpreter.evaluate(r#"
            stocks := ns("interpreter.crud.stocks")
        "#).unwrap());

        // create the table
        assert_eq!(RowsAffected(0), interpreter.evaluate(r#"
            table(symbol: String(8), exchange: String(8), last_sale: f64) ~> stocks
        "#).unwrap());

        // append a row
        assert_eq!(RowsAffected(1), interpreter.evaluate(r#"
            append stocks from { symbol: "ABC", exchange: "AMEX", last_sale: 11.77 }
        "#).unwrap());

        // write another row
        assert_eq!(RowsAffected(1), interpreter.evaluate(r#"
            { symbol: "UNO", exchange: "OTC", last_sale: 0.2456 } ~> stocks
        "#).unwrap());

        // write some more rows
        assert_eq!(RowsAffected(2), interpreter.evaluate(r#"
            [{ symbol: "BIZ", exchange: "NYSE", last_sale: 23.66 },
             { symbol: "GOTO", exchange: "OTC", last_sale: 0.1428 }] ~> stocks
        "#).unwrap());

        // write even more rows
        assert_eq!(RowsAffected(2), interpreter.evaluate(r#"
            append stocks from [
                { symbol: "BOOM", exchange: "NASDAQ", last_sale: 56.87 },
                { symbol: "TRX", exchange: "NASDAQ", last_sale: 7.9311 }
            ]
        "#).unwrap());

        // remove some rows
        assert_eq!(RowsAffected(4), interpreter.evaluate(r#"
            delete from stocks where last_sale > 1.0
        "#).unwrap());

        // overwrite a row
        assert_eq!(RowsAffected(1), interpreter.evaluate(r#"
            overwrite stocks
            via {symbol: "GOTO", exchange: "OTC", last_sale: 0.1421}
            where symbol == "GOTO"
        "#).unwrap());

        // verify the remaining rows
        assert_eq!(
            interpreter.evaluate("from stocks").unwrap(),
            TableValue(ModelRowCollection::from_rows(vec![
                make_quote(1, &phys_columns, "UNO", "OTC", 0.2456),
                make_quote(3, &phys_columns, "GOTO", "OTC", 0.1421),
            ]))
        );

        // restore the previously deleted rows
        assert_eq!(RowsAffected(4), interpreter.evaluate(r#"
            undelete from stocks where last_sale > 1.0
            "#).unwrap());

        // verify the existing rows
        assert_eq!(
            interpreter.evaluate("from stocks").unwrap(),
            TableValue(ModelRowCollection::from_rows(vec![
                make_quote(0, &phys_columns, "ABC", "AMEX", 11.77),
                make_quote(1, &phys_columns, "UNO", "OTC", 0.2456),
                make_quote(2, &phys_columns, "BIZ", "NYSE", 23.66),
                make_quote(3, &phys_columns, "GOTO", "OTC", 0.1421),
                make_quote(4, &phys_columns, "BOOM", "NASDAQ", 56.87),
                make_quote(5, &phys_columns, "TRX", "NASDAQ", 7.9311),
            ]))
        );
    }

    #[actix::test]
    async fn test_scan_from_namespace() {
        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(r#"
            [+] stocks := ns("interpreter.scan.stocks")
            [+] table(symbol: String(8), exchange: String(8), last_sale: f64) ~> stocks
            [+] [{ symbol: "ABC", exchange: "AMEX", last_sale: 12.33 },
                 { symbol: "UNO", exchange: "OTC", last_sale: 0.2456 },
                 { symbol: "BIZ", exchange: "NYSE", last_sale: 9.775 },
                 { symbol: "GOTO", exchange: "OTC", last_sale: 0.1442 },
                 { symbol: "XYZ", exchange: "NYSE", last_sale: 0.0289 }] ~> stocks
            [+] delete from stocks where last_sale > 1.0
            [+] scan stocks
        "#).unwrap();

        // |-----------------------------------------------|
        // | symbol | exchange | last_sale | _id | _active |
        // |-----------------------------------------------|
        // | ABC    | AMEX     | 12.33     | 0   | false   |
        // | UNO    | OTC      | 0.2456    | 1   | true    |
        // | BIZ    | NYSE     | 9.775     | 2   | false   |
        // | GOTO   | OTC      | 0.1442    | 3   | true    |
        // | XYZ    | NYSE     | 0.0289    | 4   | true    |
        // |-----------------------------------------------|
        let mrc = result.to_table().unwrap();
        let mrc_rows = mrc.read_active_rows().unwrap();
        let scan_columns = mrc.get_columns();
        for s in TableRenderer::from_rows(mrc_rows.clone()) { println!("{}", s); }
        assert_eq!(mrc_rows, vec![
            make_scan_quote(0, scan_columns, "ABC", "AMEX", 12.33, false),
            make_scan_quote(1, scan_columns, "UNO", "OTC", 0.2456, true),
            make_scan_quote(2, scan_columns, "BIZ", "NYSE", 9.775, false),
            make_scan_quote(3, scan_columns, "GOTO", "OTC", 0.1442, true),
            make_scan_quote(4, scan_columns, "XYZ", "NYSE", 0.0289, true),
        ])
    }

    #[actix::test]
    async fn test_select_from_namespace() {
        // create a table with test data
        let columns = make_quote_columns();
        let phys_columns = TableColumn::from_columns(&columns).unwrap();

        // append some rows
        let mut interpreter = Interpreter::new();
        assert_eq!(RowsAffected(5), interpreter.evaluate(r#"
            stocks := ns("interpreter.select1.stocks")
            table(symbol: String(8), exchange: String(8), last_sale: f64) ~> stocks
            append stocks from [
                { symbol: "ABC", exchange: "AMEX", last_sale: 11.77 },
                { symbol: "UNO", exchange: "OTC", last_sale: 0.2456 },
                { symbol: "BIZ", exchange: "NYSE", last_sale: 23.66 },
                { symbol: "GOTO", exchange: "OTC", last_sale: 0.1428 },
                { symbol: "BOOM", exchange: "NASDAQ", last_sale: 0.0872 }
            ]
        "#).unwrap());

        // compile and execute the code
        assert_eq!(interpreter.evaluate(r#"
            select symbol, exchange, last_sale
            from stocks
            where last_sale > 1.0
            order by symbol
            limit 5
        "#).unwrap(), TableValue(ModelRowCollection::from_rows(vec![
            make_quote(0, &phys_columns, "ABC", "AMEX", 11.77),
            make_quote(2, &phys_columns, "BIZ", "NYSE", 23.66),
        ])));
    }

    #[actix::test]
    async fn test_select_from_variable() {
        // create a table with test data
        let columns = make_quote_columns();
        let phys_columns = TableColumn::from_columns(&columns).unwrap();

        // set up the interpreter
        let mut interpreter = Interpreter::new();
        assert_eq!(interpreter.evaluate(r#"
            [+] stocks := ns("interpreter.select.stocks")
            [+] table(symbol: String(8), exchange: String(8), last_sale: f64) ~> stocks
            [+] append stocks from [
                    { symbol: "ABC", exchange: "AMEX", last_sale: 11.77 },
                    { symbol: "UNO", exchange: "OTC", last_sale: 0.2456 },
                    { symbol: "BIZ", exchange: "NYSE", last_sale: 23.66 },
                    { symbol: "GOTO", exchange: "OTC", last_sale: 0.1428 },
                    { symbol: "BOOM", exchange: "NASDAQ", last_sale: 0.0872 }
                ]
            [+] select symbol, exchange, last_sale
                from stocks
                where last_sale > 1.0
                order by symbol
                limit 5
        "#).unwrap(), TableValue(ModelRowCollection::from_rows(vec![
            make_quote(0, &phys_columns, "ABC", "AMEX", 11.77),
            make_quote(2, &phys_columns, "BIZ", "NYSE", 23.66),
        ])));
    }

    #[actix::test]
    async fn test_reverse_from_namespace() {
        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(r#"
            [+] stocks := ns("interpreter.reverse.stocks")
            [+] table(symbol: String(8), exchange: String(8), last_sale: f64) ~> stocks
            [+] [{ symbol: "ABC", exchange: "AMEX", last_sale: 12.49 },
                 { symbol: "BOOM", exchange: "NYSE", last_sale: 56.88 },
                 { symbol: "JET", exchange: "NASDAQ", last_sale: 32.12 }] ~> stocks
            [+] reverse from stocks
        "#).unwrap();
        let phys_columns = make_table_columns();
        assert_eq!(result, TableValue(ModelRowCollection::from_rows(vec![
            make_quote(2, &phys_columns, "JET", "NASDAQ", 32.12),
            make_quote(1, &phys_columns, "BOOM", "NYSE", 56.88),
            make_quote(0, &phys_columns, "ABC", "AMEX", 12.49),
        ])));
    }

    #[actix::test]
    async fn test_reverse_from_variable() {
        let phys_columns = make_table_columns();
        let mut interpreter = Interpreter::new();
        interpreter.with_variable("stocks", TableValue(
            ModelRowCollection::from_rows(vec![
                make_quote(0, &phys_columns, "ABC", "AMEX", 11.88),
                make_quote(1, &phys_columns, "UNO", "OTC", 0.2456),
                make_quote(2, &phys_columns, "BIZ", "NYSE", 23.66),
                make_quote(3, &phys_columns, "GOTO", "OTC", 0.1428),
                make_quote(4, &phys_columns, "BOOM", "NASDAQ", 56.87),
            ])));

        let result = interpreter.evaluate(r#"
            reverse from stocks
        "#).unwrap();
        assert_eq!(result, TableValue(ModelRowCollection::from_rows(vec![
            make_quote(4, &phys_columns, "BOOM", "NASDAQ", 56.87),
            make_quote(3, &phys_columns, "GOTO", "OTC", 0.1428),
            make_quote(2, &phys_columns, "BIZ", "NYSE", 23.66),
            make_quote(1, &phys_columns, "UNO", "OTC", 0.2456),
            make_quote(0, &phys_columns, "ABC", "AMEX", 11.88),
        ])));
    }

    #[actix::test]
    async fn test_processing_pipeline() {
        let mut interpreter = Interpreter::new();
        let result = interpreter.evaluate(r#"
            [+] stocks := ns("interpreter.pipeline.stocks")
            [+] table(symbol: String(8), exchange: String(8), last_sale: f64) ~> stocks
            [+] [{ symbol: "ABC", exchange: "AMEX", last_sale: 12.49 },
                 { symbol: "BOOM", exchange: "NYSE", last_sale: 56.88 },
                 { symbol: "JET", exchange: "NASDAQ", last_sale: 32.12 }] ~> stocks
            [+] delete from stocks where last_sale < 30.0
            [+] from stocks
        "#).unwrap();
        let phys_columns = make_table_columns();
        assert_eq!(result, TableValue(ModelRowCollection::from_rows(vec![
            make_quote(1, &phys_columns, "BOOM", "NYSE", 56.88),
            make_quote(2, &phys_columns, "JET", "NASDAQ", 32.12),
        ])));
    }

    #[actix::test]
    async fn test_type_of() {
        verify_results(r#"
            type_of(1234)
        "#, StringValue("i64".to_string()));
    }

    #[actix::test]
    async fn write_to_stderr() {
        verify_results(r#"
            stderr "Goodbye Cruel World"
        "#, Ack);
    }

    #[actix::test]
    async fn write_to_stdout() {
        verify_results(r#"
            stdout "Hello World"
        "#, Ack);
    }

    #[actix::test]
    async fn test_while_loop() {
        let mut interpreter = Interpreter::new();
        assert_eq!(Ack, interpreter.evaluate("x := 0").unwrap());
        assert_eq!(Ack, interpreter.evaluate(r#"
            while (x < 5)
                x := x + 1
        "#).unwrap());
        assert_eq!(Int64Value(5), interpreter.evaluate("x").unwrap());
    }

    fn verify_results(code: &str, expected: TypedValue) {
        let mut interpreter = Interpreter::new();
        let actual = interpreter.evaluate(code).unwrap();
        assert_eq!(actual, expected);
    }
}