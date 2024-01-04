use std::fs;

use log::{debug, info};
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use tiny_http::{Request, Response, ResponseBox};

macro_rules! to_json {
    ($obj:expr) => {{
        let json = try_unwrap!(serde_json::to_string($obj));

        Response::from_string(json)
            .with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap(),
            )
            .with_status_code(200)
            .boxed()
    }};
}

macro_rules! try_json {
    ($req:expr) => {{
        let mut content = String::new();
        if let Err(e) = $req.as_reader().read_to_string(&mut content) {
            return Response::from_string(format!("{e:?}"))
                .with_status_code(400)
                .boxed();
        }

        let result = try_unwrap!(serde_json::from_str(&content));

        result
    }};
}

macro_rules! try_unwrap {
    ($obj:expr) => {{
        match $obj {
            Ok(result) => result,
            Err(e) => {
                return Response::from_string(format!("{e:?}"))
                    .with_status_code(400)
                    .boxed()
            }
        }
    }};
}

fn get_auth(req: &Request) -> Option<(String, String)> {
    todo!()
}

macro_rules! try_auth {
    ($db:expr, $req:expr) => {{
        let Some((user, pass)) = get_auth($req) else {
            return Response::from_string("")
                .with_header(
                    tiny_http::Header::from_bytes(
                        &b"WWW-Authenticate"[..],
                        &b"Basic realm=\"my realm\""[..],
                    )
                    .unwrap(),
                )
                .with_status_code(401)
                .boxed();
        };

        let user: Option<u32> = $db
            .query_row(
                "SELECT id \
                FROM users WHERE username=?1 AND password=?2",
                params![user, pass],
                |row| Ok(row.get(0).unwrap()),
            )
            .optional()
            .unwrap();

        let Some(user) = user else {
            return Response::from_string("Invalid login")
                .with_status_code(401)
                .boxed();
        };

        user
    }};
}

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
            "api/listCategories" => return list_categories(db, req),
            "api/addCategory" => {}
            "api/listData" => return list_data(db, req),
            "api/addData" => return add_data(db, req),
            "api/removeData" => {}
            _ => {}
        }
    }

    Response::from_string("Not found")
        .with_status_code(404)
        .boxed()
}

#[derive(Deserialize)]
struct ListCategoriesRequest {
    username: String,
    include_private: bool,
}

#[derive(Serialize)]
struct ListCategoriesResponse {
    categories: Vec<Category>,
}

#[derive(Serialize)]
struct Category {
    id: u32,
    user_id: u32,
    rules: String,
    name: String,
}

fn list_categories(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let r: ListCategoriesRequest = try_json!(req);

    let auth_user_id = if r.include_private {
        Some(try_auth!(db, req))
    } else {
        None
    };

    let mut stmt = db
        .prepare(
            "SELECT c.id, c.user_id, c.rules, c.name, c.is_public, c.user_id FROM categories c \
            INNER JOIN users u ON c.user_id = u.id
            WHERE u.username = ?1",
        )
        .unwrap();

    let categories = stmt
        .query_map(params![&r.username], |row| {
            let id = row.get::<_, u32>(0).unwrap();
            let user_id = row.get::<_, u32>(1).unwrap();
            let rules = row.get::<_, String>(2).unwrap();
            let name = row.get::<_, String>(3).unwrap();
            let is_public = row.get::<_, u32>(4).unwrap() == 1;
            let is_owner = auth_user_id.map(|id| id == user_id) == Some(true);

            if !is_public && !is_owner {
                Ok(None)
            } else {
                Ok(Some(Category {
                    id,
                    user_id,
                    rules,
                    name,
                }))
            }
        })
        .unwrap()
        .collect::<Result<Vec<Option<Category>>, _>>()
        .unwrap();

    to_json!(&ListCategoriesResponse {
        categories: categories.into_iter().filter_map(|c| c).collect::<Vec<_>>()
    })
}

#[derive(Deserialize)]
struct ListDataRequest {
    category_id: u32,
}

#[derive(Serialize)]
struct ListDataResponse {
    data: Vec<Data>,
}

#[derive(Deserialize, Serialize)]
struct Data {
    time: u64,
    value: String,
}

fn list_data(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let r: ListDataRequest = try_json!(req);

    let mut stmt = db
        .prepare("SELECT e.time, e.value FROM entries e WHERE e.category_id = ?1")
        .unwrap();

    let data = stmt
        .query_map(params![&r.category_id], |row| {
            Ok(Data {
                time: row.get::<_, u64>(0).unwrap(),
                value: row.get::<_, String>(1).unwrap(),
            })
        })
        .unwrap()
        .collect::<Result<Vec<Data>, _>>()
        .unwrap();

    to_json!(&ListDataResponse { data })
}

#[derive(Deserialize)]
struct AddDataRequest {
    category_id: u32,
    data: Data,
}

fn add_data(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let user_id = try_auth!(db, req);
    let r: AddDataRequest = try_json!(req);

    let mut stmt = db
        .prepare("SELECT e.time, e.value FROM entries e WHERE e.category_id = ?1")
        .unwrap();

    let data = stmt
        .query_map(params![&r.category_id], |row| {
            Ok(Data {
                time: row.get::<_, u64>(0).unwrap(),
                value: row.get::<_, String>(1).unwrap(),
            })
        })
        .unwrap()
        .collect::<Result<Vec<Data>, _>>()
        .unwrap();

    to_json!(&ListDataResponse { data })
}
