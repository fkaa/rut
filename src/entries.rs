use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
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

pub(crate) fn add_data(
    db: &mut Connection,
    req: &mut Request,
    telegram_params: Option<&TelegramParameters>,
) -> ResponseBox {
    let (user_id, username) = crate::try_auth!(db, req);
    let r: AddDataRequest = crate::try_json!(req);

    let Some(cat) = crate::category::get_category(db, r.category_id, user_id, true) else {
        return Response::from_string("").with_status_code(400).boxed();
    };

    db.execute(
        "INSERT INTO entries (category_id, time, value)\
        VALUES (?1, ?2, ?3)",
        params![r.category_id, r.data.time, r.data.value],
    )
    .unwrap();

    if let Some(params) = telegram_params {
        if cat.is_public && cat.rules.contains("telegram=yes") {
            send_telegram_message(
                format!("{} added '{}' to {}", username, r.data.value, cat.name),
                params,
            );
        }
    }

    Response::from_string("").with_status_code(200).boxed()
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
