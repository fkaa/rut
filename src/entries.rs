use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use tiny_http::{Request, Response, ResponseBox};

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

pub(crate) fn list_data(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let r: ListDataRequest = crate::try_json!(req);

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

    crate::to_json!(&ListDataResponse { data })
}

#[derive(Deserialize)]
struct AddDataRequest {
    category_id: u32,
    data: Data,
}

pub(crate) fn add_data(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let (user_id, _) = crate::try_auth!(db, req);
    let r: AddDataRequest = crate::try_json!(req);

    let Some(cat) = crate::category::get_category(db, r.category_id, user_id, true) else {
        return Response::from_string("").with_status_code(400).boxed();
    };

    db.execute(
        "INSERT INTO entries (category_id, time, value)\
        VALUES (?1, ?2, ?3)",
        params![r.category_id, r.data.time, r.data.value]).unwrap();

    Response::from_string("").with_status_code(200).boxed()
}