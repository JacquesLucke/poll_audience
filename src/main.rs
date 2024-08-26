use actix_cors::Cors;
use actix_web::http::header::{CacheControl, CacheDirective, ContentType};
use actix_web::http::StatusCode;
use actix_web::{error, get, post, web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use derive_more::derive::{Display, Error};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    #[arg(long, default_value = "8080")]
    port: u16,
}

#[derive(Debug, Display, Error)]
enum AppError {
    EmptySessionID,
    TooLongSessionID,
    SessionIDDoesNotExist,
    ServerError,
    EmptyUserID,
    TooLongUserID,
}

impl error::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        match *self {
            AppError::EmptySessionID => StatusCode::BAD_REQUEST,
            AppError::TooLongSessionID => StatusCode::BAD_REQUEST,
            AppError::SessionIDDoesNotExist => StatusCode::BAD_REQUEST,
            AppError::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::EmptyUserID => StatusCode::BAD_REQUEST,
            AppError::TooLongUserID => StatusCode::BAD_REQUEST,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SessionID(String);

impl SessionID {
    fn from_string(s: &str) -> Result<SessionID, AppError> {
        if s.is_empty() {
            Err(AppError::EmptySessionID)
        } else if s.len() > 100 {
            Err(AppError::TooLongSessionID)
        } else {
            Ok(SessionID(s.to_string()))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
struct UserID(String);

impl UserID {
    fn from_string(s: &str) -> Result<UserID, AppError> {
        if s.is_empty() {
            Err(AppError::EmptyUserID)
        } else if s.len() > 100 {
            Err(AppError::TooLongUserID)
        } else {
            Ok(UserID(s.to_string()))
        }
    }
}

struct AppState {
    sessions: Mutex<Sessions>,
}

struct Sessions {
    state_by_id: HashMap<SessionID, SessionState>,
}

struct SessionState {
    page_content: String,
    response_by_user: HashMap<UserID, String>,
}

#[get("/")]
async fn index() -> String {
    "Missing session ID in URL".into()
}

#[get("/{session_id}")]
async fn page_for_session(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<impl Responder, AppError> {
    let session_id = SessionID::from_string(&path.into_inner())?;
    let sessions = state.sessions.lock().map_err(|_| AppError::ServerError)?;
    let session = sessions
        .state_by_id
        .get(&session_id)
        .ok_or(AppError::SessionIDDoesNotExist)?;
    Ok(HttpResponse::Ok()
        .insert_header(CacheControl(vec![CacheDirective::NoCache]))
        .body(session.page_content.clone()))
}

#[post("/{session_id}/set_page")]
async fn set_page(
    body: String,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<impl Responder, AppError> {
    let session_id = SessionID::from_string(&path.into_inner())?;
    let mut sessions = state.sessions.lock().map_err(|_| AppError::ServerError)?;
    let session = sessions
        .state_by_id
        .entry(session_id)
        .or_insert_with(|| SessionState {
            page_content: "".into(),
            response_by_user: HashMap::new(),
        });
    session.page_content = body;
    session.response_by_user.clear();
    Ok(HttpResponse::Ok())
}

#[post("/{session_id}/reset_responses")]
async fn reset(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<impl Responder, AppError> {
    let session_id = SessionID::from_string(&path.into_inner())?;
    let mut sessions = state.sessions.lock().map_err(|_| AppError::ServerError)?;
    match sessions.state_by_id.get_mut(&session_id) {
        None => {}
        Some(session) => {
            session.response_by_user.clear();
        }
    }
    Ok(HttpResponse::Ok())
}

#[post("/{session_id}/respond/{user_id}")]
async fn respond(
    body: String,
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> Result<impl Responder, AppError> {
    let (session_id, user_id) = path.into_inner();
    let session_id = SessionID::from_string(&session_id)?;
    let user_id = UserID::from_string(&user_id)?;
    let mut sessions = state.sessions.lock().map_err(|_| AppError::ServerError)?;
    let session = sessions
        .state_by_id
        .get_mut(&session_id)
        .ok_or(AppError::SessionIDDoesNotExist)?;
    session.response_by_user.insert(user_id, body);
    Ok(HttpResponse::Ok())
}

#[get("/{session_id}/responses")]
async fn responses(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<impl Responder, AppError> {
    let session_id = SessionID::from_string(&path.into_inner())?;
    let sessions = state.sessions.lock().map_err(|_| AppError::ServerError)?;
    let session = sessions
        .state_by_id
        .get(&session_id)
        .ok_or(AppError::SessionIDDoesNotExist)?;
    Ok(HttpResponse::Ok()
        .insert_header(CacheControl(vec![CacheDirective::NoCache]))
        .json(&session.response_by_user))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let state = web::Data::new(AppState {
        sessions: Mutex::new(Sessions {
            state_by_id: HashMap::new(),
        }),
    });
    println!("Start server on http://{}:{}", args.host, args.port);
    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .app_data(state.clone())
            .service(index)
            .service(page_for_session)
            .service(set_page)
            .service(respond)
            .service(responses)
            .service(reset)
    })
    .bind((args.host, args.port))?
    .run()
    .await
}
