//! Local JSON endpoints. CPU-heavy work runs outside Tokio's asynchronous workers.
use crate::simulation::{self, JobProgress, SimulationParams, SimulationSummary};
use crate::storage::{HistoricalRecord, StorageManager};
use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const DIRECT_RUN_LIMIT: u64 = 2_000_000;
const MAX_RETAINED_JOBS: usize = 64;
const JOB_RETENTION: Duration = Duration::from_secs(30 * 60);
static JOB_SEQUENCE: AtomicU64 = AtomicU64::new(0);
type ApiError = (StatusCode, String);

struct ActiveJob {
    progress: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
    total_runs: u64,
    start_time: Instant,
    is_finished: bool,
    result: Option<SimulationSummary>,
    error: Option<String>,
}

/// Shared state for one server instance; each instance needs its own CSV path.
pub struct AppState {
    pub storage: StorageManager,
    jobs: Mutex<HashMap<String, ActiveJob>>,
}

impl AppState {
    pub fn new(storage: StorageManager) -> Self {
        Self {
            storage,
            jobs: Mutex::new(HashMap::new()),
        }
    }
}

/// Assets are embedded at compile time so release downloads need only the executable.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../static/index.html")) }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../static/app.js"),
                )
            }),
        )
        .route("/api/simulate", post(simulate_handler))
        .route("/api/job/:id", get(job_status_handler))
        .route("/api/job/:id/cancel", post(cancel_job_handler))
        .route(
            "/api/history",
            get(get_history_handler).delete(clear_history_handler),
        )
        .route("/api/export_csv", get(export_csv_handler))
        .with_state(state)
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum SimulateResponse {
    Direct(Box<SimulationSummary>),
    AsyncJob { job_id: String, status: String },
}

fn internal_error(message: impl ToString) -> ApiError {
    (StatusCode::INTERNAL_SERVER_ERROR, message.to_string())
}

async fn simulate_handler(
    State(state): State<Arc<AppState>>,
    Json(params): Json<SimulationParams>,
) -> Result<Json<SimulateResponse>, ApiError> {
    params
        .validate()
        .map_err(|message| (StatusCode::BAD_REQUEST, message))?;
    // Survival and target modes can be slow even for small batches. They always
    // return a job so the browser can show progress and request cancellation.
    let background = params.runs > DIRECT_RUN_LIMIT
        || matches!(params.mode.as_str(), "target_profit" | "till_bust");
    if !background {
        let summary = tokio::task::spawn_blocking(move || {
            let summary = simulation::run_monte_carlo_stream(
                &params,
                Arc::new(AtomicU64::new(0)),
                Arc::new(AtomicBool::new(false)),
            )
            .ok_or_else(|| "Simulation cancelled".to_string())?;
            state.storage.save_record(&summary)?;
            Ok::<_, String>(summary)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;
        return Ok(Json(SimulateResponse::Direct(Box::new(summary))));
    }

    let job_id = format!("job_{}", JOB_SEQUENCE.fetch_add(1, Ordering::Relaxed));
    let progress = Arc::new(AtomicU64::new(0));
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut jobs = state.jobs.lock().await;
        jobs.retain(|_, job| !job.is_finished || job.start_time.elapsed() < JOB_RETENTION);
        // Evict the oldest completed result when full; active jobs stay addressable.
        if jobs.len() >= MAX_RETAINED_JOBS {
            let oldest = jobs
                .iter()
                .filter(|(_, job)| job.is_finished)
                .min_by_key(|(_, job)| job.start_time)
                .map(|(id, _)| id.clone());
            if let Some(id) = oldest {
                jobs.remove(&id);
            } else {
                return Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Too many active jobs".into(),
                ));
            }
        }
        jobs.insert(
            job_id.clone(),
            ActiveJob {
                progress: progress.clone(),
                cancel: cancel.clone(),
                total_runs: params.runs,
                start_time: Instant::now(),
                is_finished: false,
                result: None,
                error: None,
            },
        );
    }

    let worker_state = state.clone();
    let worker_id = job_id.clone();
    tokio::spawn(async move {
        let storage_state = worker_state.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            let summary = simulation::run_monte_carlo_stream(&params, progress, cancel.clone());
            let summary = if cancel.load(Ordering::Relaxed) {
                None
            } else {
                summary
            };
            if let Some(summary) = &summary {
                storage_state.storage.save_record(summary)?;
            }
            Ok::<_, String>(summary)
        })
        .await;
        let mut jobs = worker_state.jobs.lock().await;
        if let Some(job) = jobs.get_mut(&worker_id) {
            match outcome {
                Ok(Ok(summary)) => job.result = summary,
                Ok(Err(message)) => job.error = Some(message),
                Err(error) => job.error = Some(format!("Simulation task failed: {error}")),
            }
            job.is_finished = true;
        }
    });

    Ok(Json(SimulateResponse::AsyncJob {
        job_id,
        status: "running".into(),
    }))
}

async fn job_status_handler(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<JobProgress>, ApiError> {
    let jobs = state.jobs.lock().await;
    let job = jobs
        .get(&job_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Job not found".into()))?;
    let completed = job.progress.load(Ordering::Relaxed).min(job.total_runs);
    let elapsed = job.start_time.elapsed().as_secs_f64();
    let speed = if elapsed > 0.001 {
        completed as f64 / elapsed
    } else {
        0.0
    };
    let estimated_remaining_ms = if speed > 0.0 && !job.is_finished {
        job.total_runs.saturating_sub(completed) as f64 / speed * 1000.0
    } else {
        0.0
    };
    Ok(Json(JobProgress {
        job_id,
        total_runs: job.total_runs,
        completed_runs: completed,
        percentage: completed as f64 / job.total_runs as f64 * 100.0,
        speed_runs_per_sec: speed,
        elapsed_ms: elapsed * 1000.0,
        estimated_remaining_ms,
        is_finished: job.is_finished,
        is_cancelled: job.cancel.load(Ordering::Relaxed) && job.result.is_none(),
        result: job.result.clone(),
        error: job.error.clone(),
    }))
}

async fn cancel_job_handler(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let jobs = state.jobs.lock().await;
    let job = jobs
        .get(&job_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Job not found".into()))?;
    if job.is_finished {
        return Err((StatusCode::CONFLICT, "Job has already finished".into()));
    }
    job.cancel.store(true, Ordering::Relaxed);
    Ok(Json(
        serde_json::json!({ "status": "cancelling", "job_id": job_id }),
    ))
}

async fn get_history_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<HistoricalRecord>>, ApiError> {
    state
        .storage
        .get_all_records()
        .map(Json)
        .map_err(internal_error)
}

async fn clear_history_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.storage.clear_records().map_err(internal_error)?;
    Ok(Json(
        serde_json::json!({ "status": "success", "message": "History cleared" }),
    ))
}

async fn export_csv_handler(State(state): State<Arc<AppState>>) -> Result<Response, ApiError> {
    let csv = state.storage.export_csv().map_err(internal_error)?;
    let mut response = csv.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"martingale_simulations.csv\""),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{params, TestDirectory};
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn app(dir: &TestDirectory) -> Router {
        create_router(Arc::new(AppState::new(StorageManager::with_path(
            dir.csv_path(),
        ))))
    }
    async fn request(
        app: &Router,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, String, axum::http::HeaderMap) {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let text = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        (status, text, headers)
    }
    #[tokio::test]
    async fn assets_and_csv_download_have_correct_content_types() {
        let dir = TestDirectory::new();
        let app = app(&dir);
        let (status, html, _) = request(&app, "GET", "/", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains("lang=\"en\""));
        let (status, js, headers) = request(&app, "GET", "/app.js", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(js.contains("setSimulationMode"));
        assert_eq!(
            headers[header::CONTENT_TYPE],
            "text/javascript; charset=utf-8"
        );
        let (status, csv, headers) = request(&app, "GET", "/api/export_csv", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(csv.starts_with("id,timestamp,mode"));
        assert!(headers[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .contains("attachment"));
    }
    #[tokio::test]
    async fn simulate_history_export_and_clear_work_together() {
        let dir = TestDirectory::new();
        let app = app(&dir);
        let mut p = params();
        p.start_capital = 4.0;
        p.unlimited_max_bet = true;
        let (status, body, _) = request(
            &app,
            "POST",
            "/api/simulate",
            Some(serde_json::to_value(p).unwrap()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let result: SimulationSummary = serde_json::from_str(&body).unwrap();
        assert_eq!(result.bankrupt_runs, 8);
        let (_, history, _) = request(&app, "GET", "/api/history", None).await;
        let history: serde_json::Value = serde_json::from_str(&history).unwrap();
        assert_eq!(history.as_array().unwrap().len(), 1);
        assert_eq!(history[0]["max_bet"], 0.0);
        assert_eq!(
            request(&app, "DELETE", "/api/history", None).await.0,
            StatusCode::OK
        );
        assert_eq!(request(&app, "GET", "/api/history", None).await.1, "[]");
    }
    #[tokio::test]
    async fn invalid_payloads_are_rejected_without_writing_history() {
        let dir = TestDirectory::new();
        let app = app(&dir);
        for (key, value) in [
            ("runs", serde_json::json!(0)),
            ("mode", serde_json::json!("unknown")),
            ("roulette_type", serde_json::json!("unknown")),
            ("rounds_per_run", serde_json::json!(0)),
        ] {
            let mut p = serde_json::to_value(params()).unwrap();
            p[key] = value;
            assert_eq!(
                request(&app, "POST", "/api/simulate", Some(p)).await.0,
                StatusCode::BAD_REQUEST
            );
        }
        assert!(!dir.csv_path().exists());
        assert_eq!(
            request(&app, "POST", "/api/simulate", Some(serde_json::json!({})))
                .await
                .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    #[tokio::test]
    async fn asynchronous_job_finishes_and_saves_history() {
        let dir = TestDirectory::new();
        let app = app(&dir);
        let mut p = params();
        p.mode = "till_bust".into();
        p.start_capital = 4.0;
        let (_, body, _) = request(
            &app,
            "POST",
            "/api/simulate",
            Some(serde_json::to_value(p).unwrap()),
        )
        .await;
        let value: serde_json::Value = serde_json::from_str(&body).unwrap();
        let path = format!("/api/job/{}", value["job_id"].as_str().unwrap());
        let progress = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let (_, body, _) = request(&app, "GET", &path, None).await;
                let progress: JobProgress = serde_json::from_str(&body).unwrap();
                if progress.is_finished {
                    break progress;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        assert!(progress.error.is_none());
        assert_eq!(progress.completed_runs, 8);
        assert_eq!(progress.result.unwrap().bankrupt_runs, 8);
        assert_eq!(
            request(&app, "POST", &format!("{path}/cancel"), None)
                .await
                .0,
            StatusCode::CONFLICT
        );
    }
    #[tokio::test]
    async fn unknown_jobs_return_not_found() {
        let dir = TestDirectory::new();
        let app = app(&dir);
        assert_eq!(
            request(&app, "GET", "/api/job/missing", None).await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            request(&app, "POST", "/api/job/missing/cancel", None)
                .await
                .0,
            StatusCode::NOT_FOUND
        );
    }
    #[tokio::test]
    async fn persistence_failure_is_reported_instead_of_success() {
        let dir = TestDirectory::new();
        let app = create_router(Arc::new(AppState::new(StorageManager::with_path(
            dir.0.join("missing/history.csv"),
        ))));
        let (status, body, _) = request(
            &app,
            "POST",
            "/api/simulate",
            Some(serde_json::to_value(params()).unwrap()),
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(body.contains("CSV"));
    }
}

#[cfg(test)]
mod background_tests {
    use super::*;
    use crate::test_support::{params, TestDirectory};
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    async fn send(
        app: &Router,
        method: &str,
        path: &str,
        payload: Option<serde_json::Value>,
    ) -> serde_json::Value {
        let req = Request::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(payload.map_or_else(Body::empty, |v| Body::from(v.to_string())))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap()
    }
    async fn finished(app: &Router, path: &str) -> serde_json::Value {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let progress = send(app, "GET", path, None).await;
                if progress["is_finished"] == true {
                    break progress;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap()
    }
    #[tokio::test]
    async fn cancelled_background_job_discards_partial_results() {
        let dir = TestDirectory::new();
        let app = create_router(Arc::new(AppState::new(StorageManager::with_path(
            dir.csv_path(),
        ))));
        let mut p = params();
        p.runs = simulation::MAX_RUNS;
        p.mode = "till_bust".into();
        p.start_capital = 4.0;
        let job = send(
            &app,
            "POST",
            "/api/simulate",
            Some(serde_json::to_value(p).unwrap()),
        )
        .await;
        let path = format!("/api/job/{}", job["job_id"].as_str().unwrap());
        send(&app, "POST", &format!("{path}/cancel"), None).await;
        let progress = finished(&app, &path).await;
        assert_eq!(progress["is_cancelled"], true);
        assert!(progress["result"].is_null());
        assert!(!dir.csv_path().exists());
    }
    #[tokio::test]
    async fn background_storage_error_finishes_with_an_error() {
        let dir = TestDirectory::new();
        let app = create_router(Arc::new(AppState::new(StorageManager::with_path(
            dir.0.join("missing/history.csv"),
        ))));
        let mut p = params();
        p.mode = "till_bust".into();
        p.start_capital = 4.0;
        let job = send(
            &app,
            "POST",
            "/api/simulate",
            Some(serde_json::to_value(p).unwrap()),
        )
        .await;
        let path = format!("/api/job/{}", job["job_id"].as_str().unwrap());
        let progress = finished(&app, &path).await;
        assert!(progress["error"].as_str().unwrap().contains("CSV"));
        assert!(progress["result"].is_null());
    }
}
