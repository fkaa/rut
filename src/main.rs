use std::fs;

use log::{debug, info};
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use tiny_http::{Request, Response, ResponseBox};

mod macros;
use macros::*;

mod category;
mod entries;

const MIGRATIONS: [M; 1] = [M::up(include_str!("../migrations/0001_initial.sql"))];

fn main() {
    env_logger::init();

    info!("Starting rut");

    let mut db = Connection::open("db.sqlite3").unwrap();
    let migrations = Migrations::new(MIGRATIONS.to_vec());
    migrations
        .to_latest(&mut db)
        .expect("Failed to apply migrations");

    let server = tiny_http::Server::http("127.0.0.1:8000").unwrap();

    info!("Listening for HTTP requests...");
    for mut req in server.incoming_requests() {
        let response = get_response(&mut db, &mut req);

        debug!(
            "{} {} => {}",
            req.method(),
            req.url(),
            response.status_code().0
        );

        let _ = req.respond(response);
    }
}

fn get_response(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let url = req.url();
    if url.ends_with("/") || url.ends_with("/index.html") {
        let content = fs::read("index.html").unwrap();
        return Response::from_data(content).with_status_code(200).boxed();
    }

    if let Some((_, path)) = url.split_once("/") {
        match path {
            "api/login" => return login(db, req),
            "api/listCategories" => return category::list_categories(db, req),
            "api/addCategory" => return category::add_category(db, req),
            "api/editCategory" => return category::edit_category(db, req),
            "api/removeCategory" => return category::remove_category(db, req),
            "api/listData" => return entries::list_data(db, req),
            "api/addData" => return entries::add_data(db, req),
            "api/removeData" => {}
            _ => {}
        }
    }

    Response::from_string("Not found")
        .with_status_code(404)
        .boxed()
}

fn login(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let (_id, user) = try_auth!(db, req);

    #[derive(Serialize)]
    struct LoginResponse {
        user: String,
    }

    to_json!(&LoginResponse { user })
}


