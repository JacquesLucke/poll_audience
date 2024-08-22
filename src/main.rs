use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use std::collections::HashMap;
use std::sync::Mutex;

type SessionID = String;

struct AppState {
    sessions: Mutex<HashMap<SessionID, SessionState>>,
}

struct SessionState {
    page_content: String,
    responses: Vec<String>,
}

#[get("/")]
async fn index() -> String {
    "Hello World".into()
}

#[get("/{session_id}")]
async fn page_for_session(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let session_id = path.into_inner();
    let sessions = state.sessions.lock().unwrap();
    match sessions.get(&session_id) {
        None => HttpResponse::NotFound().body("Unknown Session\n"),
        Some(session) => HttpResponse::Ok().body(session.page_content.clone()),
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
        responses: Vec::new(),
    });
    session.page_content = body;
    session.responses.clear();
    "Page Updated\n"
}

#[post("/{session_id}/respond")]
async fn respond(
    body: String,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let session_id = path.into_inner();
    let mut sessions = state.sessions.lock().unwrap();
    match sessions.get_mut(&session_id) {
        None => HttpResponse::NotFound().body("Unknown Session\n"),
        Some(session) => {
            session.responses.push(body);
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
        Some(session) => HttpResponse::Ok().json(&session.responses),
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
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
