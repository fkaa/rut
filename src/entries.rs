use crate::require;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::thread;
use tiny_http::{Request, Response, ResponseBox};

use crate::TelegramParameters;

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
    id: u32,
    time: u64,
    value: String,
}

pub(crate) fn list_data(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let r: ListDataRequest = crate::try_json!(req);

    let Some(cat) = crate::category::get_category(db, r.category_id) else {
        return Response::from_string("").with_status_code(404).boxed();
    };

    if !cat.is_public {
        let (user_id, username) = crate::try_auth!(db, req);
        if cat.user_id != user_id {
            return Response::from_string("").with_status_code(403).boxed();
        }
    }

    let mut stmt = db
        .prepare("SELECT e.id, e.time, e.value FROM entries e WHERE e.category_id = ?1")
        .unwrap();

    let data = stmt
        .query_map(params![&r.category_id], |row| {
            Ok(Data {
                id: row.get::<_, u32>(0).unwrap(),
                time: row.get::<_, u64>(1).unwrap(),
                value: row.get::<_, String>(2).unwrap(),
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
    data: AddData,
}

#[derive(Deserialize, Serialize)]
struct AddData {
    time: u64,
    value: String,
}

pub(crate) fn add_data(
    db: &mut Connection,
    req: &mut Request,
    telegram_params: Option<&TelegramParameters>,
) -> ResponseBox {
    let (user_id, username) = crate::try_auth!(db, req);
    let r: AddDataRequest = crate::try_json!(req);

    require!(r.data.value.len() < 1024);

    let Some(cat) = crate::category::get_category(db, r.category_id) else {
        return Response::from_string("").with_status_code(400).boxed();
    };

    if cat.user_id != user_id {
        return Response::from_string("").with_status_code(403).boxed();
    }

    db.execute(
        "INSERT INTO entries (category_id, time, value)\
        VALUES (?1, ?2, ?3)",
        params![r.category_id, r.data.time, r.data.value],
    )
    .unwrap();

    if let Some(params) = telegram_params {
        if cat.is_public && cat.rules.contains("telegram=yes") {
            let params = params.clone();

            thread::spawn(move || {
                send_telegram_message(
                    format!("{} added '{}' to {}", username, r.data.value, cat.name),
                    &params,
                );
            });
        }
    }

    Response::from_string("{}").with_status_code(200).boxed()
}

static BASE_API_URL: &str = "https://api.telegram.org/bot";

fn send_telegram_message(msg: String, params: &TelegramParameters) {
    use frankenstein::{ureq, Api, ChatId, SendMessageParams, TelegramApi};
    use std::time::Duration;

    let request_agent = ureq::builder().timeout(Duration::from_secs(100)).build();
    let api_url = format!("{BASE_API_URL}{}", params.telegram_token);

    let api = Api::builder()
        .api_url(api_url)
        .request_agent(request_agent)
        .build();

    let _ = api.send_message(&SendMessageParams {
        chat_id: ChatId::Integer(params.telegram_chat),
        message_thread_id: None,
        text: msg,
        parse_mode: None,
        entities: None,
        link_preview_options: None,
        disable_notification: None,
        protect_content: None,
        reply_parameters: None,
        reply_markup: None,
    });
}

#[derive(Deserialize)]
struct EditDataRequest {
    category_id: u32,
    data_id: u32,
    new_value: String,
}

pub(crate) fn edit_data(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let (user_id, _) = crate::try_auth!(db, req);
    let r: EditDataRequest = crate::try_json!(req);

    require!(r.new_value.len() < 1024);

    let Some(cat) = crate::category::get_category(db, r.category_id) else {
        return Response::from_string("").with_status_code(400).boxed();
    };

    if cat.user_id != user_id {
        return Response::from_string("").with_status_code(403).boxed();
    }

    let rows = db
        .execute(
            "UPDATE entries SET value=?3\
        WHERE category_id=?1 AND id=?2",
            params![r.category_id, r.data_id, r.new_value],
        )
        .unwrap();

    if rows == 1 {
        Response::from_string("{}").with_status_code(200).boxed()
    } else {
        Response::from_string("").with_status_code(404).boxed()
    }
}

#[derive(Deserialize)]
struct RemoveDataRequest {
    category_id: u32,
    data_id: u32,
}

pub(crate) fn remove_data(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let (user_id, _) = crate::try_auth!(db, req);
    let r: crate::entries::RemoveDataRequest = crate::try_json!(req);

    let Some(cat) = crate::category::get_category(db, r.category_id) else {
        return Response::from_string("").with_status_code(400).boxed();
    };

    if cat.user_id != user_id {
        return Response::from_string("").with_status_code(403).boxed();
    }

    let rows = db
        .execute(
            "DELETE FROM entries \
        WHERE category_id=?1 AND id=?2",
            params![r.category_id, r.data_id],
        )
        .unwrap();

    if rows == 1 {
        Response::from_string("{}").with_status_code(200).boxed()
    } else {
        Response::from_string("").with_status_code(404).boxed()
    }
}
