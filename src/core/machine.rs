////////////////////////////////////////////////////////////////////
// state machine module
////////////////////////////////////////////////////////////////////

use std::collections::HashMap;
use std::sync::Mutex;

use actix::Addr;
use log::error;
use serde::{Deserialize, Serialize};
use tokio::io;

use shared_lib::fail;

use crate::dataframe_actor::DataframeActor;
use crate::expression::Expression;
use crate::expression::Expression::Ns;
use crate::namespaces::Namespace;
use crate::read_fully;
use crate::rows::Row;
use crate::typed_values::TypedValue;
use crate::typed_values::TypedValue::*;

/// returns a filtered collection of rows based on the supplied condition.
macro_rules! filter_rows {
    ($ms:expr, $condition:expr, $rows:expr, $limit:expr, $f:expr) => {
        for row in $rows {
            if $ms.is_true(&row, $condition) { $f(row.clone()); }
        }
    }
}

/// represents the state of the machine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MachineState {
    stack: Vec<TypedValue>,
    variables: HashMap<String, TypedValue>,
}

impl MachineState {
    /// creates a new state machine
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            variables: HashMap::new(),
        }
    }

    pub fn get_variables(&self) -> &HashMap<String, TypedValue> {
        &self.variables
    }

    /// evaluates the specified [Expression]; returning a [TypedValue] result.
    pub fn evaluate(&self, expression: &Expression) -> io::Result<(MachineState, TypedValue)> {
        use crate::expression::Expression::*;
        match expression {
            And(a, b) =>
                self.expand2(a, b, |aa, bb| aa.and(&bb).unwrap_or(Undefined)),
            Between(a, b, c) =>
                self.expand3(a, b, c, |aa, bb, cc| Boolean((aa >= bb) && (aa <= cc))),
            CodeBlock(ops) => self.evaluate_all(ops),
            Delete { table, condition, limit } =>
                self.sql_delete(table, condition, limit),
            Divide(a, b) =>
                self.expand2(a, b, |aa, bb| aa / bb),
            Equal(a, b) =>
                self.expand2(a, b, |aa, bb| Boolean(aa == bb)),
            Factorial(a) => self.expand1(a, |aa| aa.factorial().unwrap_or(Undefined)),
            GreaterThan(a, b) =>
                self.expand2(a, b, |aa, bb| Boolean(aa > bb)),
            GreaterOrEqual(a, b) =>
                self.expand2(a, b, |aa, bb| Boolean(aa >= bb)),
            LessThan(a, b) =>
                self.expand2(a, b, |aa, bb| Boolean(aa < bb)),
            LessOrEqual(a, b) =>
                self.expand2(a, b, |aa, bb| Boolean(aa <= bb)),
            Literal(value) => Ok((self.clone(), value.clone())),
            Minus(a, b) =>
                self.expand2(a, b, |aa, bb| aa - bb),
            Modulo(a, b) =>
                self.expand2(a, b, |aa, bb| aa % bb),
            Multiply(a, b) =>
                self.expand2(a, b, |aa, bb| aa * bb),
            NotEqual(a, b) =>
                self.expand2(a, b, |aa, bb| Boolean(aa != bb)),
            Or(a, b) =>
                self.expand2(a, b, |aa, bb| aa.or(&bb).unwrap_or(Undefined)),
            Not(a) => self.expand1(a, |aa| !aa),
            Plus(a, b) =>
                self.expand2(a, b, |aa, bb| aa + bb),
            Pow(a, b) =>
                self.expand2(a, b, |aa, bb| aa.pow(&bb).unwrap_or(Undefined)),
            Range(a, b) =>
                self.expand2(a, b, |aa, bb| aa.range(&bb).unwrap_or(Undefined)),
            Select { fields, from, condition, group_by, having, order_by, limit } =>
                self.sql_select(fields, from, condition, group_by, having, order_by, limit),
            SetVariable(name, e) => {
                let (ms, result) = self.evaluate(e)?;
                Ok((ms.set(name, result), Undefined))
            }
            ShiftLeft(a, b) =>
                self.expand2(a, b, |aa, bb| aa << bb),
            ShiftRight(a, b) =>
                self.expand2(a, b, |aa, bb| aa >> bb),
            Tuple(values) => self.evaluate_array(values),
            Variable(name) => Ok((self.clone(), self.get(&name).unwrap_or(Undefined))),
            Xor(a, b) =>
                self.expand2(a, b, |aa, bb| aa ^ bb),
            other => fail(format!("Unhandled expression: {:?}", other))
        }
    }

    /// evaluates the specified [Expression]; returning a [TypedValue] result.
    pub fn evaluate_all(&self, ops: &Vec<Expression>) -> io::Result<(MachineState, TypedValue)> {
        Ok(ops.iter().fold((self.clone(), Undefined),
                           |(ms, _), op| match ms.evaluate(op) {
                               Ok((ms, tv)) => (ms, tv),
                               Err(err) => panic!("{}", err.to_string())
                           }))
    }


    /// evaluates the specified [Expression]; returning an array ([TypedValue]) result.
    pub fn evaluate_array(&self, ops: &Vec<Expression>) -> io::Result<(MachineState, TypedValue)> {
        let (ms, results) = ops.iter()
            .fold((self.clone(), vec![]),
                  |(ms, mut array), op| match ms.evaluate(op) {
                      Ok((ms, tv)) => {
                          array.push(tv);
                          (ms, array)
                      }
                      Err(err) => panic!("{}", err.to_string())
                  });
        Ok((ms, Array(results)))
    }

    /// executes the specified instructions on this state machine.
    pub fn execute(&self, ops: &Vec<OpCode>) -> io::Result<(MachineState, TypedValue)> {
        let mut ms = self.clone();
        for op in ops { ms = op(&ms)? }
        let (ms, result) = ms.pop_or(Undefined);
        Ok((ms, result))
    }

    fn expand(ms: MachineState, expr: &Option<Box<Expression>>) -> io::Result<(MachineState, TypedValue)> {
        match expr {
            Some(item) => ms.evaluate(item),
            None => Ok((ms, Undefined))
        }
    }

    fn expand_vec(ms: MachineState, expr: &Option<Vec<Expression>>) -> io::Result<(MachineState, TypedValue)> {
        match expr {
            Some(items) => ms.evaluate_array(items),
            None => Ok((ms, Undefined))
        }
    }

    /// evaluates the boxed expression and applies the supplied function
    fn expand1(&self,
               a: &Box<Expression>,
               f: fn(TypedValue) -> TypedValue) -> io::Result<(MachineState, TypedValue)> {
        let (ms, aa) = self.evaluate(a)?;
        Ok((ms, f(aa)))
    }

    /// evaluates the two boxed expressions and applies the supplied function
    fn expand2(&self,
               a: &Box<Expression>,
               b: &Box<Expression>,
               f: fn(TypedValue, TypedValue) -> TypedValue) -> io::Result<(MachineState, TypedValue)> {
        let (ms, aa) = self.evaluate(a)?;
        let (ms, bb) = ms.evaluate(b)?;
        Ok((ms, f(aa, bb)))
    }

    /// evaluates the three boxed expressions and applies the supplied function
    fn expand3(&self,
               a: &Box<Expression>,
               b: &Box<Expression>,
               c: &Box<Expression>,
               f: fn(TypedValue, TypedValue, TypedValue) -> TypedValue) -> io::Result<(MachineState, TypedValue)> {
        let (ms, aa) = self.evaluate(a)?;
        let (ms, bb) = ms.evaluate(b)?;
        let (ms, cc) = ms.evaluate(c)?;
        Ok((ms, f(aa, bb, cc)))
    }

    /// returns a variable by name
    pub fn get(&self, name: &str) -> Option<TypedValue> {
        self.variables.get(name).map(|x| x.clone())
    }

    /// returns the option of a value from the stack
    pub fn pop(&self) -> (Self, Option<TypedValue>) {
        let mut stack = self.stack.clone();
        let value = stack.pop();
        let variables = self.variables.clone();
        (MachineState { stack, variables }, value)
    }

    /// returns a value from the stack or the default value if the stack is empty.
    pub fn pop_or(&self, default_value: TypedValue) -> (Self, TypedValue) {
        let (ms, result) = self.pop();
        (ms, result.unwrap_or(default_value))
    }

    /// pushes a value unto the stack
    pub fn push(&self, value: TypedValue) -> Self {
        let mut stack = self.stack.clone();
        stack.push(value);
        MachineState {
            stack,
            variables: self.variables.clone(),
        }
    }

    /// pushes a collection of values unto the stack
    pub fn push_all(&self, values: Vec<TypedValue>) -> Self {
        values.iter().fold(self.clone(), |ms, tv| ms.push(tv.clone()))
    }

    fn filter_rows_in_memory(&self,
                             condition: &Option<Box<Expression>>,
                             rows: Vec<Row>,
                             limit: TypedValue) -> io::Result<(MachineState, TypedValue)> {
        let ms = self.clone();
        let mut out = vec![];
        filter_rows!(ms, condition.clone(), rows, limit, |row| out.push(row));
        Ok((self.clone(), ArrayOfRows(out)))
    }

    async fn filter_rows_on_disk(&self,
                                 ns: Namespace,
                                 condition: &Option<Box<Expression>>,
                                 actor: Addr<DataframeActor>,
                                 limit: TypedValue) -> io::Result<(MachineState, TypedValue)> {
        let ms = self.clone();
        let mut out = vec![];
        let rows = read_fully!(actor, ns)?;
        filter_rows!(ms, condition.clone(), rows, limit, |row: Row| out.push(row.clone()));
        Ok((self.clone(), ArrayOfRows(out)))
    }

    fn is_true(&self, row: &Row, condition: Option<Box<Expression>>) -> bool {
        condition.is_none() || condition.is_some_and(|expr|
            match self.with_row(row).evaluate(&expr) {
                Ok((_, result)) => result == Boolean(true),
                Err(err) => {
                    error!("{}", err);
                    false
                }
            })
    }

    pub fn set(&self, name: &str, value: TypedValue) -> Self {
        let mut variables = self.variables.clone();
        variables.insert(name.to_string(), value);
        MachineState {
            stack: self.stack.clone(),
            variables,
        }
    }

    fn sql_delete(&self,
                  from: &Box<Expression>,
                  condition: &Option<Box<Expression>>,
                  limit: &Option<Box<Expression>>) -> io::Result<(MachineState, TypedValue)> {
        let (ms, _limit) = Self::expand(self.clone(), limit)?;
        match Self::expand(ms, &Some(from.clone()))? {
            //(ms, ArrayOfRows(rows)) => ms.process_delete_rows(condition, rows, _limit),
            (_, x) => fail(format!("Type mismatch: expected an iterable - {:?}", x))
        }
    }

    fn sql_select(&self,
                  fields: &Vec<Expression>,
                  from: &Option<Box<Expression>>,
                  condition: &Option<Box<Expression>>,
                  group_by: &Option<Vec<Expression>>,
                  having: &Option<Box<Expression>>,
                  order_by: &Option<Vec<Expression>>,
                  limit: &Option<Box<Expression>>) -> io::Result<(MachineState, TypedValue)> {
        let (ms, _limit) = Self::expand(self.clone(), limit)?;
        match from.clone() {
            None => ms.evaluate_array(fields),
            Some(my_from) => {
                match *my_from {
                    Ns(ns) => {
                        //ms.filter_rows_on_disk(ns, condition, actor, _limit);
                        Ok((ms, Undefined))
                    }
                    _ =>
                        match Self::expand(ms, from)? {
                            (ms, Undefined) => ms.evaluate_array(fields),
                            (ms, ArrayOfRows(rows)) => ms.filter_rows_in_memory(condition, rows, _limit),
                            (_, x) => fail(format!("Type mismatch: expected an iterable - {:?}", x))
                        }
                }
            }
        }
    }

    pub fn stack_len(&self) -> usize {
        self.stack.len()
    }

    pub fn transform_numeric(&self, number: TypedValue,
                             fi: fn(i64) -> TypedValue,
                             ff: fn(f64) -> TypedValue) -> io::Result<Self> {
        match number {
            Float32Value(n) => Ok(self.push(ff(n as f64))),
            Float64Value(n) => Ok(self.push(ff(n))),
            Int8Value(n) => Ok(self.push(fi(n as i64))),
            Int16Value(n) => Ok(self.push(fi(n as i64))),
            Int32Value(n) => Ok(self.push(fi(n as i64))),
            Int64Value(n) => Ok(self.push(fi(n))),
            unknown => fail(format!("Unsupported type {:?}", unknown))
        }
    }

    pub fn with_row(&self, row: &Row) -> Self {
        row.fields.iter().zip(row.columns.iter())
            .fold(self.clone(), |ms, (f, c)| {
                ms.with_variable(c.get_name(), f.value.clone())
            })
    }

    pub fn with_variable(&self, name: &str, value: TypedValue) -> Self {
        let mut variables = self.variables.clone();
        variables.insert(name.to_string(), value);
        MachineState {
            stack: self.stack.clone(),
            variables,
        }
    }
}

/// represents an executable instruction (opcode)
pub type OpCode = fn(&MachineState) -> io::Result<MachineState>;

// Unit tests
#[cfg(test)]
mod tests {
    use crate::compiler::Compiler;
    use crate::expression::{FALSE, NULL, TRUE};
    use crate::expression::Expression::{Divide, Factorial, Literal, Variable};
    use crate::row;
    use crate::table_columns::TableColumn;
    use crate::testdata::make_columns;

    use super::*;

    #[test]
    fn test_compile_and_evaluate_all_n_pow_2() {
        let ms = MachineState::new().set("n", Int64Value(5));
        let opcodes = Compiler::compile("n ** 2").unwrap();
        let (_ms, result) = ms.evaluate_all(&opcodes).unwrap();
        assert_eq!(result, Int64Value(25))
    }

    #[test]
    fn test_compile_and_evaluate_all_n_gt_5() {
        let ms = MachineState::new().set("n", Int64Value(7));
        let opcodes = Compiler::compile("n > 5").unwrap();
        let (_ms, result) = ms.evaluate_all(&opcodes).unwrap();
        assert_eq!(result, Boolean(true))
    }

    #[test]
    fn test_evaluate_all_factorial() {
        let ms = MachineState::new();
        let opcodes = vec![
            Factorial(Box::new(Literal(Float64Value(6.))))
        ];
        let (_ms, result) = ms.evaluate_all(&opcodes).unwrap();
        assert_eq!(result, Float64Value(720.))
    }

    #[test]
    fn test_evaluate_array() {
        let ms = MachineState::new();
        let (_, array) =
            ms.evaluate_array(&vec![Literal(Float64Value(3.25)), TRUE, FALSE, NULL]).unwrap();
        assert_eq!(array, Array(vec![Float64Value(3.25), Boolean(true), Boolean(false), Null]));
    }

    #[test]
    fn test_push_all() {
        let ms = MachineState::new().push_all(vec![
            Float32Value(2.), Float64Value(3.),
            Int16Value(4), Int32Value(5),
            Int64Value(6), StringValue("Hello World".into()),
        ]);
        assert_eq!(ms.stack, vec![
            Float32Value(2.), Float64Value(3.),
            Int16Value(4), Int32Value(5),
            Int64Value(6), StringValue("Hello World".into()),
        ])
    }

    #[test]
    fn test_sql_select() {
        let ops = Compiler::compile(r#"
            select symbol, exchange, lastSale from stocks
            where lastSale < 1.0
            order by symbol
            limit 5
            "#).unwrap();
        let columns = make_columns();
        let phys_columns = TableColumn::from_columns(&columns).unwrap();
        let ms = MachineState::new()
            .with_variable("stocks", ArrayOfRows(vec![
                row!(0, phys_columns, vec![
                    StringValue("ABC".into()), StringValue("AMEX".into()), Float64Value(11.77),
                ]),
                row!(1, phys_columns, vec![
                    StringValue("UNO".into()), StringValue("OTC".into()), Float64Value(0.2456),
                ]),
                row!(2, phys_columns, vec![
                    StringValue("BIZ".into()), StringValue("NYSE".into()), Float64Value(23.66),
                ]),
                row!(3, phys_columns, vec![
                    StringValue("GOTO".into()), StringValue("OTC".into()), Float64Value(0.1428),
                ]),
                row!(4, phys_columns, vec![
                    StringValue("BOOM".into()), StringValue("NASD".into()), Float64Value(56.87),
                ]),
            ]));
        let (_, result) = ms.evaluate_all(&ops).unwrap();
        assert_eq!(result, ArrayOfRows(vec![
            row!(1, phys_columns, vec![
                StringValue("UNO".into()), StringValue("OTC".into()), Float64Value(0.2456),
            ]),
            row!(3, phys_columns, vec![
                StringValue("GOTO".into()), StringValue("OTC".into()), Float64Value(0.1428),
            ]),
        ]));
    }

    #[test]
    fn test_variables_directly_1() {
        let ms = MachineState::new()
            .set("abc", Int32Value(5))
            .set("xyz", Int32Value(58));
        assert_eq!(ms.get("abc"), Some(Int32Value(5)));
        assert_eq!(ms.get("xyz"), Some(Int32Value(58)));
    }

    #[test]
    fn test_variables_directly_2() {
        let ms = MachineState::new()
            .set("x", Int64Value(50));
        let (ms, result) = ms.evaluate(
            &Divide(Box::new(Variable("x".into())), Box::new(Literal(Int64Value(7))))
        ).unwrap();
        assert_eq!(result, Int64Value(7));
        assert_eq!(ms.get("x"), Some(Int64Value(50)));
    }

    #[test]
    fn test_variables_indirectly() {
        let ops = Compiler::compile(r#"
            x := 5
            y := 7
            x * y
        "#).unwrap();
        let ms = MachineState::new();
        let (_, result) = ms.evaluate_all(&ops).unwrap();
        assert_eq!(result, Int64Value(35));
    }

    #[ignore]
    #[test]
    fn test_precedence() {
        let ms = MachineState::new();
        let opcodes = Compiler::compile("2 + 4 * 3").unwrap();
        let (_ms, result) = ms.evaluate_all(&opcodes).unwrap();
        assert_eq!(result, Float64Value(14.))
    }
}