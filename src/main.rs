use actix_cors::Cors;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use std::collections::HashMap;
use std::sync::Mutex;

type SessionID = String;

struct AppState {
    sessions: Mutex<HashMap<SessionID, SessionState>>,
}

struct SessionState {
    page_content: String,
    response_by_user: HashMap<String, String>,
}

#[get("/")]
async fn index() -> String {
    "Missing session ID in URL".into()
}

#[get("/{session_id}")]
async fn page_for_session(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let session_id = path.into_inner();
    let sessions = state.sessions.lock().unwrap();
    match sessions.get(&session_id) {
        None => HttpResponse::NotFound().body("Unknown Session\n"),
        Some(session) => HttpResponse::Ok()
            .insert_header(CacheControl(vec![CacheDirective::NoCache]))
            .body(session.page_content.clone()),
    }
}

#[post("/{session_id}/set_page")]
async fn set_page(
    body: String,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let session_id = path.into_inner();
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions.entry(session_id).or_insert_with(|| SessionState {
        page_content: "".into(),
        response_by_user: HashMap::new(),
    });
    session.page_content = body;
    session.response_by_user.clear();
    "Page Updated\n"
}

#[post("/{session_id}/respond/{user_id}")]
async fn respond(
    body: String,
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> impl Responder {
    let (session_id, user_id) = path.into_inner();
    let mut sessions = state.sessions.lock().unwrap();
    match sessions.get_mut(&session_id) {
        None => HttpResponse::NotFound().body("Unknown Session\n"),
        Some(session) => {
            session.response_by_user.insert(user_id, body);
            HttpResponse::Ok().body("Responded\n")
        }
    }
}

#[get("/{session_id}/responses")]
async fn responses(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let session_id = path.into_inner();
    let sessions = state.sessions.lock().unwrap();
    match sessions.get(&session_id) {
        None => HttpResponse::NotFound().json(Vec::<String>::new()),
        Some(session) => HttpResponse::Ok()
            .insert_header(CacheControl(vec![CacheDirective::NoCache]))
            .json(&session.response_by_user),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        sessions: Mutex::new(HashMap::new()),
    });
    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .app_data(state.clone())
            .service(index)
            .service(page_for_session)
            .service(set_page)
            .service(respond)
            .service(responses)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
