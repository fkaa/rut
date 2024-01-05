use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use tiny_http::{Request, Response, ResponseBox};

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
pub struct Category {
    pub id: u32,
    pub user_id: u32,
    pub rules: String,
    pub name: String,
    pub is_public: bool
}

pub(crate) fn list_categories(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let r: ListCategoriesRequest = crate::try_json!(req);

    let mut auth_user_id = None;

    if r.include_private && auth_user_id.is_none() {
        let (user_id, _username) = crate::try_auth!(db, req);

        auth_user_id = Some(user_id);
    }

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
                    is_public
                }))
            }
        })
        .unwrap()
        .collect::<Result<Vec<Option<Category>>, _>>()
        .unwrap();

    crate::to_json!(&ListCategoriesResponse {
        categories: categories.into_iter().filter_map(|c| c).collect::<Vec<_>>()
    })
}

#[derive(Deserialize)]
struct AddCategoryRequest {
    name: String,
    rules: String,
    is_public: bool,
}

pub(crate) fn add_category(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let (user_id, _) = crate::try_auth!(db, req);
    let r: AddCategoryRequest = crate::try_json!(req);

    db.execute(
        "INSERT INTO categories (user_id, name, rules, is_public)\
        VALUES (?1, ?2, ?3, ?4)",
        params![user_id, r.name, r.rules, if r.is_public { 1 } else { 0 }]).unwrap();

    Response::from_string("{}").with_status_code(200).boxed()
}

#[derive(Deserialize)]
struct RemoveCategoryRequest {
    category_id: u32,
}

pub(crate) fn remove_category(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let (user_id, _) = crate::try_auth!(db, req);
    let r: RemoveCategoryRequest = crate::try_json!(req);

    db.execute(
        "DELETE FROM categories \
        WHERE id = ?1 AND user_id = ?2",
        params![r.category_id, user_id]).unwrap();

    Response::from_string("{}").with_status_code(200).boxed()
}

pub(crate) fn get_category(db: &Connection, category_id: u32, user_id: u32, authed: bool) -> Option<Category> {
    db.query_row(
        "SELECT c.id, c.user_id, c.rules, c.name, c.is_public, c.user_id FROM categories c \
        INNER JOIN users u ON c.user_id = u.id
        WHERE c.id == ?1 AND c.user_id = ?2", params![category_id, user_id], |row| {
            let id = row.get::<_, u32>(0).unwrap();
            let category_user_id = row.get::<_, u32>(1).unwrap();
            let rules = row.get::<_, String>(2).unwrap();
            let name = row.get::<_, String>(3).unwrap();
            let is_public = row.get::<_, u32>(4).unwrap() == 1;
            let is_owner = category_user_id == user_id && authed;

            if !is_public && !is_owner {
                return Ok(None);
            }

            Ok(Some(Category {
                id,
                user_id,
                rules,
                name,
                is_public,
            }))
        },
    ).unwrap()
}

#[derive(Deserialize)]
struct EditCategoryRequest {
    category_id: u32,
    name: String,
    rules: String,
    is_public: bool,
}

pub(crate) fn edit_category(db: &mut Connection, req: &mut Request) -> ResponseBox {
    let (user_id, _) = crate::try_auth!(db, req);
    let r: EditCategoryRequest = crate::try_json!(req);

    let Some(cat) = crate::category::get_category(db, r.category_id, user_id, true) else {
        return Response::from_string("").with_status_code(400).boxed();
    };

    db.execute(
        "UPDATE categories SET name=?1, rules=?2, is_public=?3\
        WHERE id=?4",
        params![r.name, r.rules, &if r.is_public { 1 } else { 0 }, r.category_id]).unwrap();

    Response::from_string("{}").with_status_code(200).boxed()
}