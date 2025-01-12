#![warn(dead_code)]
////////////////////////////////////////////////////////////////////
// Oxide Websockets module
////////////////////////////////////////////////////////////////////

use crate::byte_code_compiler::ByteCodeCompiler;
use crate::errors::throw;
use crate::errors::Errors::Exact;
use crate::expression::Expression;
use crate::interpreter::Interpreter;
use crate::typed_values::TypedValue;
use crate::typed_values::TypedValue::{ErrorValue, StringValue, Undefined};
use actix::{Actor, StreamHandler};
use actix_web_actors::ws;
use actix_web_actors::ws::WebsocketContext;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use shared_lib::cnv_error;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

/// Oxide WebSocket Client
pub struct OxideWebSocketClient {
    read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
}

impl OxideWebSocketClient {
    /// Starts the websocket client
    pub async fn connect(host: &str, port: u16) -> std::io::Result<OxideWebSocketClient> {
        let (mut ws_stream, _response) =
            connect_async(format!("ws://{host}:{port}/ws")).await
                .map_err(|e| cnv_error!(e))?;
        let (mut write, mut read) = ws_stream.split();
        Ok(Self { read, write })
    }

    pub async fn evaluate(&mut self, script: &str) -> std::io::Result<TypedValue> {
        self.send_text_message(script).await?;
        self.read_next().await
    }

    pub async fn invoke(&mut self, expr: &Expression) -> std::io::Result<TypedValue> {
        self.send_binary_message(ByteCodeCompiler::encode(expr)?).await?;
        self.read_next().await
    }

    pub async fn with_variable(&mut self, name: &str, value: TypedValue) -> std::io::Result<TypedValue> {
        self.send_text_message(format!("{name} := {}", value.to_code()).as_str()).await?;
        self.read_next().await
    }

    async fn read_next(&mut self) -> std::io::Result<TypedValue> {
        match self.read.next().await {
            None => Ok(Undefined),
            Some(Ok(message)) =>
                Ok(match message {
                    Message::Binary(bytes) => ByteCodeCompiler::decode_value(&bytes),
                    Message::Text(text) => StringValue(text.to_string()),
                    msg => ErrorValue(Exact(format!("Unexpected result: {}", msg)))
                }),
            Some(Err(err)) => throw(Exact(err.to_string()))
        }
    }

    async fn send_binary_message(&mut self, message: Vec<u8>) -> std::io::Result<()> {
        self.write.send(Message::Binary(message)).await
            .map_err(|e| cnv_error!(e))
    }

    async fn send_text_message(&mut self, message: &str) -> std::io::Result<()> {
        self.write.send(Message::Text(message.to_string())).await
            .map_err(|e| cnv_error!(e))
    }
}

/// Oxide WebSocket Server
pub struct OxideWebSocketServer {
    interpreter: Interpreter,
}

impl OxideWebSocketServer {
    pub fn new() -> Self {
        Self { interpreter: Interpreter::new() }
    }
}

impl Actor for OxideWebSocketServer {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for OxideWebSocketServer {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        /// transmits the [TypedValue] to the client
        fn transmit(ctx: &mut WebsocketContext<OxideWebSocketServer>, value: &TypedValue) {
            let bytes = ByteCodeCompiler::encode_value(&value)
                .unwrap_or_else(|err| {
                    eprintln!("ERROR: {}", err);
                    vec![]
                });
            ctx.binary(bytes);
        }

        match msg {
            Err(err) => transmit(ctx, &ErrorValue(Exact(err.to_string()))),
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(msg)) => ctx.ping(&msg),
            Ok(ws::Message::Text(text)) => {
                let value = self.interpreter.evaluate(text.trim_ascii())
                    .unwrap_or_else(|err| ErrorValue(Exact(err.to_string())));
                transmit(ctx, &value)
            }
            Ok(ws::Message::Binary(bytes)) => {
                let model = ByteCodeCompiler::decode(&bytes.into());
                let value = self.interpreter.invoke(&model)
                    .unwrap_or_else(|err| ErrorValue(Exact(err.to_string())));
                transmit(ctx, &value)
            }
            Ok(ws::Message::Close(reason)) => {
                println!("Close! [{:?}]", reason);
            }
            _ => {}
        }
    }
}

/// Unit tests
#[cfg(test)]
mod tests {
    use crate::dataframe::Dataframe::Model;
    use crate::model_row_collection::ModelRowCollection;
    use crate::numbers::Numbers::I64Value;
    use crate::oxide_server::start_http_server;
    use crate::repl;
    use crate::testdata::{make_quote, make_quote_columns};
    use crate::typed_values::TypedValue;
    use crate::typed_values::TypedValue::{Number, TableValue};
    use crate::websockets::OxideWebSocketClient;

    #[actix::test]
    async fn test_websockets_conversational() {
        let port = 8010;
        start_http_server(port);

        let mut wsc = OxideWebSocketClient::connect("0.0.0.0", port).await.unwrap();
        wsc.evaluate("a := [0, 1, 3, 5]").await.unwrap();
        let value = wsc.evaluate("a[2]").await.unwrap();
        show_value(value.clone());
        assert_eq!(value, Number(I64Value(3)))
    }

    #[actix::test]
    async fn test_websockets_script() {
        let port = 8011;
        start_http_server(port);

        let mut wsc = OxideWebSocketClient::connect("0.0.0.0", port).await.unwrap();
        let value = wsc.evaluate(r#"
            stocks := ns("ws.script.stocks")
            table(symbol: String(8), exchange: String(8), last_sale: f64) ~> stocks
            append stocks from [
                { symbol: "ABC", exchange: "AMEX", last_sale: 11.77 },
                { symbol: "UNO", exchange: "OTC", last_sale: 0.2456 },
                { symbol: "BIZ", exchange: "NYSE", last_sale: 23.66 },
                { symbol: "GOTO", exchange: "OTC", last_sale: 0.1428 },
                { symbol: "BOOM", exchange: "NASDAQ", last_sale: 0.0872 }
            ]
            from stocks
        "#).await.unwrap();
        show_value(value.clone());
        assert_eq!(value, TableValue(Model(ModelRowCollection::from_columns_and_rows(
            &make_quote_columns(), &vec![
                make_quote(0, "ABC", "AMEX", 11.77),
                make_quote(1, "UNO", "OTC", 0.2456),
                make_quote(2, "BIZ", "NYSE", 23.66),
                make_quote(3, "GOTO", "OTC", 0.1428),
                make_quote(4, "BOOM", "NASDAQ", 0.0872)
            ]
        ))))
    }

    fn show_value(value: TypedValue) {
        for s in repl::build_output(1, value, 0.33).unwrap() {
            println!("{}", s)
        }
    }
}