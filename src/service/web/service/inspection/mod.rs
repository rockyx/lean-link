use actix_web::scope;
use serde::Deserialize;
use uuid::Uuid;

pub mod station;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestStationRequest {
    pub station_id: Uuid,
    pub image_path: String,
}

#[scope("/inspection")]
pub mod api {
    use actix_web::{get, post, web};

    use crate::{
        AppState, errors,
        service::{inspection::config::InspectionSettings, web::service::WebResponse},
    };

    use super::TestStationRequest;

    #[post("/initialize")]
    pub async fn initialize(
        app_state: web::Data<AppState>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        app_state
            .inspection_manager
            .initialize_from_database()
            .await?;

        Ok(WebResponse::with_result(()).into())
    }

    #[post("/settings")]
    pub async fn set_inspection(
        app_state: web::Data<AppState>,
        req: web::Json<InspectionSettings>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        app_state.inspection_manager.set_inspection(&req).await?;

        Ok(WebResponse::with_result(()).into())
    }

    #[get("/settings")]
    pub async fn get_inspection(
        app_state: web::Data<AppState>,
    ) -> actix_web::Result<web::Json<WebResponse<InspectionSettings>>, errors::Error> {
        let inspection = app_state.inspection_manager.get_inspection().await;

        Ok(WebResponse::with_result(inspection).into())
    }

    #[post("/test-station")]
    pub async fn test_station(
        app_state: web::Data<AppState>,
        req: web::Json<TestStationRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        app_state
            .inspection_manager
            .test_station(&req.station_id, &req.image_path)
            .await?;

        Ok(WebResponse::with_result(()).into())
    }
}
