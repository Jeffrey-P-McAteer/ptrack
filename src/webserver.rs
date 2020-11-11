
use actix::{
  Actor, StreamHandler, Handler
};
use actix_web::{
  web, App, Error, HttpRequest, HttpResponse, HttpServer
};
use actix_web_actors::ws;
use actix_rt;
use actix_files;

use actix_derive::{Message};

use actix::prelude::*;

use rust_embed::RustEmbed;

use serde_json;
use serde_json::json;

use acme_client;

use std::env;
use std::sync::{
  Mutex
};
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::io::prelude::*;

use crate::gui;

#[derive(RustEmbed)]
#[folder = "src/www"]
struct WWWAssets;

#[derive(Message)]
#[rtype(result = "()")]
pub enum WsMessage {
  S(String),
  B(Vec<u8>)
}

/// Define HTTP actor
/// One of these is made for each websocket connection
struct APodWs {
  // Index in GlobalData.clients
  pub num: usize,
  // Set on connection, is true if localhost
  pub is_leader: bool,
  // A pointer to all the other clients via GlobalData
  pub data: web::Data<Mutex<GlobalData>>,
}

impl APodWs {
  pub fn new(data: web::Data<Mutex<GlobalData>>) -> Self {
    let num = data.lock().unwrap().clients.len();
    APodWs {
      num: num,
      is_leader: false,
      data: data
    }
  }
}


impl Actor for APodWs {
    type Context = ws::WebsocketContext<Self>;
}

impl Handler<WsMessage> for APodWs {
    type Result = ();
    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) -> Self::Result {
        // Occurs when a client tells the server something + the server broadcasts.
        // We must forward "msg" to the client's websocket connection.
        match msg {
          WsMessage::S(msg) => {
            ctx.text(msg);
          }
          WsMessage::B(bin) => {
            ctx.binary(bin);
          }
        }
    }
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for APodWs {
    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();
        self.data.lock().unwrap().clients.push(
          addr.recipient()
        );
    }

    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => handle_ws_msg(self, ctx, text),
            Ok(ws::Message::Binary(bin)) => (),
            _ => (),
        }
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
      ctx.stop();
    }
}

struct GlobalData {
  pub clients: Vec<Recipient<WsMessage>>,
  pub save_dir: PathBuf,
}

impl Default for GlobalData {
  fn default() -> Self {
    println!("GlobalData::default run!");
    let mut save_dir = env::current_dir().expect("Could not get current_dir");
    if let Some(arg) = env::args().skip(1).next() {
      save_dir = PathBuf::from(arg);
    }
    GlobalData {
      clients: vec![],
      save_dir: save_dir
    }
  }
}

fn handle_ws_msg(ws: &mut APodWs, ctx: &mut ws::WebsocketContext<APodWs>, text: String) {
  //println!("ws text={}", &text[..]);
  // Parse JSON
  let json: serde_json::Result<serde_json::Value> = serde_json::from_str(&text[..]);
  let json = match json {
    Err(_e) => { return; },
    Ok(j) => j,
  };

  // Handle JSON for server + return
  if false {

    return;
  }

  // Anytime someone sends the server data we forward it to everyone else
  let clients = &mut ws.data.lock().unwrap().clients;
  let mut idx_to_rm: Option<usize> = None;
  for i in 0..clients.len() {
    if let Err(e) = clients[i].try_send(WsMessage::S(text.clone())) {
      println!("Error sending text to client: {}", e);
      idx_to_rm = Some(i);
    }
  }
  if let Some(idx_to_rm) = idx_to_rm {
    clients.remove(idx_to_rm);
  }

}

// This fn upgrades /ws/ http requests to a websocket connection
// which may stream events to/from the GUI
async fn ws_handler(req: HttpRequest, stream: web::Payload, data: web::Data<Mutex<GlobalData>>) -> Result<HttpResponse, Error> {
    let mut apod_ws = APodWs::new(data);
    if let Some(addr) = req.peer_addr() {
      apod_ws.is_leader = addr.ip().is_loopback();
    }
    let resp = ws::start(apod_ws, &req, stream);
    //println!("{:?}", resp);
    resp
}

// This fn grabs assets and returns them
fn index(req: HttpRequest, _stream: web::Payload) -> HttpResponse {
  
  // We perform some common routing tactics here
  let mut r_path = req.path();
  if r_path == "/" {
    r_path = "index.html";
  }
  if r_path.starts_with("/") {
    r_path = &r_path[1..];
  }
  //println!("r_path={}", &r_path);

  // Do some security checks (only localhost should talk to "leader.html")
  if r_path == "leader.html" {
    if let Some(addr) = req.peer_addr() {
      if ! addr.ip().is_loopback() {
        // Security error, don't let anyone become the leader!
        return HttpResponse::NotFound()
          .content_type("text/html")
          .body(&include_bytes!("www/404.html")[..]);
      }
    }
  }

  // Finally pull from fs/memory 
  match WWWAssets::get(r_path) {
    Some(data) => {
      // Figure out MIME from file extension
      let guess = mime_guess::from_path(r_path);
      let mime_s = guess.first_raw().unwrap_or("application/octet-stream");
      let owned_data: Vec<u8> = (&data[..]).iter().cloned().collect();
      HttpResponse::Ok()
            .content_type(mime_s)
            .body(owned_data)
    }
    None => {
      HttpResponse::NotFound()
        .content_type("text/html")
        .body(&include_bytes!("www/404.html")[..])
    }
  }
}

// This expects check-in form POST data
fn check_in(req: HttpRequest, body: web::Bytes, data: web::Data<Mutex<GlobalData>>) -> HttpResponse {
  use std::process::Command;


  HttpResponse::Ok()
      .content_type("text/plain")
      .body(r#"Data received!"#)
}

pub fn main() -> Result<(), Box<dyn std::error::Error>>  {

  let sys = actix_rt::System::new(crate::APP_NAME);
  
  let address = format!("0.0.0.0:{}", crate::HTTP_PORT);

  // First get temp dir + check for SSL certs
  let fqdn = env::var("PTRACK_FQDN").unwrap_or("publicip.jmcateer.pw");
  let cert_pem_f = {
    let mut f = std::env::temp_dir();
    f.push("ptrack_certificate.pem");
    f
  };
  let cert_key_f = {
    let mut f = std::env::temp_dir();
    f.push("ptrack_certificate.key");
    f
  };

  if (!cert_pem_f.as_path().exists()) || (!cert_key_f.as_path().exists()) {
    // Perform ACME key request
    let directory = acme_client::Directory::lets_encrypt()?;
    let account = directory.account_registration().register()?;

    // Create a identifier authorization for example.com
    let authorization = account.authorization(fqdn)?;

    // Validate ownership of example.com with http challenge
    let http_challenge = authorization.get_http_challenge().ok_or("HTTP challenge not found")?;
    // Puts a file under std::env::temp_dir() / .well-known/acme-challenge/
    // We must serve this over port 80 for the challenge to work
    http_challenge.save_key_authorization(std::env::temp_dir())?;

    // Spin up small Http server over port 80 (will need admin / root)

    let handler = std::thread::spawn(|| {
      println!("ACME HTTP server starting...");
      HttpServer::new(||  App::new().service(actix_files::Files::new("/", std::env::temp_dir()).show_files_listing()) )
          .bind("0.0.0.0:80")?
          .run();
      println!("ACME HTTP server stopped");
    });

    http_challenge.validate()?;

    let cert = account.certificate_signer(&[fqdn]).sign_certificate()?;
    cert.save_signed_certificate(cert_pem_f)?;
    cert.save_private_key(cert_key_f)?;

    actix::Arbiter::system().do_send(actix::msgs::SystemExit(0));

    handler.join()?;

  }

  let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
  
  let private_k = PKey::from_rsa(
    Rsa::private_key_from_pem( std::fs::read(&cert_key_f).unwrap() ).unwrap()
  ).unwrap();
  let cert = openssl::x509::X509::from_pem(
    std::fs::read(&cert_pem_f).unwrap()
  ).unwrap();

  builder
    .set_private_key(&private_k)
    .unwrap();
  builder
    .set_certificate(&cert)
    .unwrap();

  HttpServer::new(||
      App::new()
        .app_data( web::Data::new( Mutex::new( GlobalData::default() ) ) )
        .data(web::PayloadConfig::new(1000000)) // allow 1mb data sent to us
        .route("/ws", web::get().to(ws_handler))
        .route("/check-in", web::post().to(check_in))
        .route("/", web::get().to(index))
        .default_service(
          web::route().to(index)
        )
    )
    .workers(1)
    .backlog(16)
    //.bind(&address)?
    .bind_openssl(&address, builder)?
    .run();

  let x = sys.run()?;
  println!("x={:?}", x); // paranoia about smart compiler optimizations

  Ok(())
}

