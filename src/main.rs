use std::{env, fs};

use log::{debug, info};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tiny_http::{Request, Response, ResponseBox};

mod macros;

use macros::*;

mod category;
mod entries;

const MIGRATIONS: [M; 1] = [M::up(include_str!("../migrations/0001_initial.sql"))];

#[derive(Clone)]
pub struct TelegramParameters {
    pub telegram_token: String,
    pub telegram_chat: i64,
}

fn main() {
    env_logger::init();

    info!("Starting rut");

    let mut db = Connection::open("db.sqlite3").unwrap();
    let migrations = Migrations::new(MIGRATIONS.to_vec());
    migrations
        .to_latest(&mut db)
        .expect("Failed to apply migrations");

    let mut args = env::args();
    let _ = args.next();
    if let Some(arg) = args.next() {
        if arg == "useradd" {
            let user = args.next().expect("Expected username");
            let pass = args.next().expect("Expected password");

            let mut hasher = Sha256::new();
            hasher.update(pass.as_bytes());
            let hashed_pass = hasher.finalize();
            let base64_pass = base64::encode(hashed_pass);

            db.execute(
                "INSERT INTO users (username, password) VALUES (?1, ?2)",
                params![user, base64_pass],
            )
            .unwrap();

            log::info!("Added new user '{user}'")
        }

        return;
    }

    let telegram_params = if let (Ok(telegram_token), Ok(telegram_chat)) =
        (env::var("TELEGRAM_TOKEN"), env::var("TELEGRAM_CHAT"))
    {
        Some(TelegramParameters {
            telegram_token,
            telegram_chat: telegram_chat.parse().unwrap(),
        })
    } else {
        None
    };

    let server = tiny_http::Server::http("127.0.0.1:8000").unwrap();

    info!("Listening for HTTP requests...");
    for mut req in server.incoming_requests() {
        let response = get_response(&mut db, &mut req, telegram_params.as_ref());

        debug!(
            "{} {} => {}",
            req.method(),
            req.url(),
            response.status_code().0
        );

        let _ = req.respond(response);
    }
}

fn get_response(
    db: &mut Connection,
    req: &mut Request,
    telegram_params: Option<&TelegramParameters>,
) -> ResponseBox {
    let url = req.url();
    if url.ends_with("/") || url.ends_with("/index.html") {
        let content = fs::read("index.html").unwrap();
        return Response::from_data(content).with_status_code(200).boxed();
    }

    if let Some((_, path)) = url.split_once("/") {
        match path {
            "api/login" => return login(db, req),
            "api/updatePassword" => return update_password(db, req),
            "api/listCategories" => return category::list_categories(db, req),
            "api/addCategory" => return category::add_category(db, req),
            "api/editCategory" => return category::edit_category(db, req),
            "api/removeCategory" => return category::remove_category(db, req),
            "api/listData" => return entries::list_data(db, req),
            "api/addData" => return entries::add_data(db, req, telegram_params),
            "api/editData" => return entries::edit_data(db, req),
            "api/removeData" => return entries::remove_data(db, req),
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

#[derive(Deserialize)]
struct ChangePasswordRequest {
    new_password: String,
}

fn update_password(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let (id, user) = try_auth!(db, req);
    let r: ChangePasswordRequest = try_json!(req);

    require!(r.new_password.len() > 0);
    require!(r.new_password.len() < 1000);

    let mut hasher = Sha256::new();
    hasher.update(r.new_password.as_bytes());
    let hashed_pass = hasher.finalize();
    let base64_pass = base64::encode(hashed_pass);

    db.execute(
        "UPDATE users SET password=?1 WHERE id=?2",
        params![base64_pass, id],
    )
    .unwrap();

    Response::from_string("{}").with_status_code(200).boxed()
}
