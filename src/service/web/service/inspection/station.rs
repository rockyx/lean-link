use crate::errors;
use crate::service::inspection::manager::{
    RoiCreateRequest, RoiUpdateRequest, StationCreateRequest, StationResponse,
    StationUpdateRequest,
};
use crate::service::web::service::{ErrorCode, WebResponse};
use actix_web::{delete, get, post, put, scope, web};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ==================== Request/Response DTOs ====================

#[derive(Serialize, Deserialize, Debug)]
pub struct StationListRequest {
    pub enabled: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct StationIdPath {
    pub id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct RoiIdPath {
    pub roi_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct StationRoiPath {
    pub station_id: Uuid,
    pub roi_id: Uuid,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetEnabledRequest {
    pub enabled: bool,
}

// ==================== API Routes ====================

#[scope("/inspection/station")]
pub mod api {
    use crate::{AppState, service::inspection::station::RoiConfig};

    use super::*;

    /// Initialize stations from database
    #[post("/initialize")]
    pub async fn initialize(
        app_state: web::Data<AppState>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        app_state
            .station_manager
            .initialize_from_database()
            .await?;

        Ok(WebResponse::with_result(()).into())
    }

    /// List all stations
    #[get("/list")]
    pub async fn list(
        app_state: web::Data<AppState>,
        query: web::Query<StationListRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<Vec<StationResponse>>>, errors::Error> {
        let stations = if query.enabled.unwrap_or(false) {
            app_state.station_manager.get_enabled_stations().await
        } else {
            app_state.station_manager.get_all_stations()
        };

        let responses: Vec<StationResponse> = stations.into_iter().map(|s| s.into()).collect();
        Ok(WebResponse::with_result(responses).into())
    }

    /// Get a station by ID
    #[get("/get/{id}")]
    pub async fn get(
        app_state: web::Data<AppState>,
        path: web::Path<StationIdPath>,
    ) -> actix_web::Result<web::Json<WebResponse<StationResponse>>, errors::Error> {
        let id = path.id;

        let station = app_state
            .station_manager
            .get_station(id)
            .ok_or_else(|| errors::Error::BadRequest(ErrorCode::NotFound, "工作站不存在".into()))?;

        Ok(WebResponse::with_result(station.into()).into())
    }

    /// Create a new station
    #[post("/create")]
    pub async fn create(
        app_state: web::Data<AppState>,
        req: web::Json<StationCreateRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<String>>, errors::Error> {
        let id = app_state
            .station_manager
            .create_station(req.into_inner())
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to create station");
                errors::Error::InternalError(ErrorCode::InternalError)
            })?;

        Ok(WebResponse::with_result(id.to_string()).into())
    }

    /// Update a station
    #[put("/update/{id}")]
    pub async fn update(
        app_state: web::Data<AppState>,
        path: web::Path<StationIdPath>,
        req: web::Json<StationUpdateRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        let id = path.id;

        let updated = app_state
            .station_manager
            .update_station(id, req.into_inner())
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to update station");
                errors::Error::InternalError(ErrorCode::InternalError)
            })?;

        if !updated {
            return Err(errors::Error::BadRequest(
                ErrorCode::NotFound,
                "工作站不存在".into(),
            ));
        }

        Ok(WebResponse::with_result(()).into())
    }

    /// Delete a station
    #[delete("/delete/{id}")]
    pub async fn delete(
        app_state: web::Data<AppState>,
        path: web::Path<StationIdPath>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        let id = path.id;

        let deleted = app_state
            .station_manager
            .delete_station(id)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to delete station");
                errors::Error::InternalError(ErrorCode::InternalError)
            })?;

        if !deleted {
            return Err(errors::Error::BadRequest(
                ErrorCode::NotFound,
                "工作站不存在".into(),
            ));
        }

        Ok(WebResponse::with_result(()).into())
    }

    /// Set station enabled status
    #[post("/set-enabled/{id}")]
    pub async fn set_enabled(
        app_state: web::Data<AppState>,
        path: web::Path<StationIdPath>,
        req: web::Json<SetEnabledRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        let id = path.id;

        let update_req = StationUpdateRequest {
            is_enabled: Some(req.enabled),
            ..Default::default()
        };

        let updated = app_state
            .station_manager
            .update_station(id, update_req)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to set station enabled");
                errors::Error::InternalError(ErrorCode::InternalError)
            })?;

        if !updated {
            return Err(errors::Error::BadRequest(
                ErrorCode::NotFound,
                "工作站不存在".into(),
            ));
        }

        Ok(WebResponse::with_result(()).into())
    }

    // ==================== ROI Routes ====================

    /// Get ROIs for a station
    #[get("/{station_id}/roi/list")]
    pub async fn list_rois(
        app_state: web::Data<AppState>,
        path: web::Path<StationRoiPath>,
    ) -> actix_web::Result<web::Json<WebResponse<Vec<RoiConfig>>>, errors::Error> {
        let station_id = path.station_id;

        let rois = app_state
            .station_manager
            .get_station_rois(station_id)
            .ok_or_else(|| errors::Error::BadRequest(ErrorCode::NotFound, "工作站不存在".into()))?;

        Ok(WebResponse::with_result(rois).into())
    }

    /// Add ROI to a station
    #[post("/{station_id}/roi/create")]
    pub async fn create_roi(
        app_state: web::Data<AppState>,
        path: web::Path<StationRoiPath>,
        req: web::Json<RoiCreateRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<String>>, errors::Error> {
        let station_id = path.station_id;

        let roi_id = app_state
            .station_manager
            .add_roi(station_id, req.into_inner())
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to create ROI");
                errors::Error::InternalError(ErrorCode::InternalError)
            })?;

        Ok(WebResponse::with_result(roi_id.to_string()).into())
    }

    /// Update ROI
    #[put("/{station_id}/roi/update/{roi_id}")]
    pub async fn update_roi(
        app_state: web::Data<AppState>,
        path: web::Path<StationRoiPath>,
        req: web::Json<RoiUpdateRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        let roi_id = path.roi_id;

        let updated = app_state
            .station_manager
            .update_roi(roi_id, req.into_inner())
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to update ROI");
                errors::Error::InternalError(ErrorCode::InternalError)
            })?;

        if !updated {
            return Err(errors::Error::BadRequest(
                ErrorCode::NotFound,
                "ROI不存在".into(),
            ));
        }

        Ok(WebResponse::with_result(()).into())
    }

    /// Delete ROI
    #[delete("/{station_id}/roi/delete/{roi_id}")]
    pub async fn delete_roi(
        app_state: web::Data<AppState>,
        path: web::Path<StationRoiPath>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        let roi_id = path.roi_id;

        let deleted = app_state
            .station_manager
            .delete_roi(roi_id)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to delete ROI");
                errors::Error::InternalError(ErrorCode::InternalError)
            })?;

        if !deleted {
            return Err(errors::Error::BadRequest(
                ErrorCode::NotFound,
                "ROI不存在".into(),
            ));
        }

        Ok(WebResponse::with_result(()).into())
    }
}
