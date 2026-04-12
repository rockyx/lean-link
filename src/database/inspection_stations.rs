use crate::database::entity::{PageResult, prelude::*, t_inspection_stations, t_station_rois};
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, InsertResult,
    PaginatorTrait, QueryFilter, QueryOrder,
};
use uuid::Uuid;

// ============================================================================
// Inspection Station CRUD Operations
// ============================================================================

pub async fn insert_inspection_station(
    conn: &DatabaseConnection,
    station: t_inspection_stations::ActiveModel,
) -> Result<InsertResult<t_inspection_stations::ActiveModel>, DbErr> {
    TInspectionStations::insert(station).exec(conn).await
}

pub async fn find_inspection_station_by_id(
    conn: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<t_inspection_stations::Model>, DbErr> {
    TInspectionStations::find()
        .filter(
            t_inspection_stations::Column::Id
                .eq(id)
                .and(t_inspection_stations::Column::DeletedAt.is_null()),
        )
        .one(conn)
        .await
}

pub async fn find_all_inspection_stations(
    conn: &DatabaseConnection,
) -> Result<Vec<t_inspection_stations::Model>, DbErr> {
    TInspectionStations::find()
        .filter(t_inspection_stations::Column::DeletedAt.is_null())
        .order_by_asc(t_inspection_stations::Column::CreatedAt)
        .all(conn)
        .await
}

pub async fn find_inspection_stations_by_enabled(
    conn: &DatabaseConnection,
    enabled: bool,
) -> Result<Vec<t_inspection_stations::Model>, DbErr> {
    TInspectionStations::find()
        .filter(
            t_inspection_stations::Column::IsEnabled
                .eq(enabled)
                .and(t_inspection_stations::Column::DeletedAt.is_null()),
        )
        .order_by_asc(t_inspection_stations::Column::CreatedAt)
        .all(conn)
        .await
}

pub async fn find_inspection_stations_by_camera_id(
    conn: &DatabaseConnection,
    camera_id: Uuid,
) -> Result<Vec<t_inspection_stations::Model>, DbErr> {
    TInspectionStations::find()
        .filter(
            t_inspection_stations::Column::CameraId
                .eq(camera_id)
                .and(t_inspection_stations::Column::DeletedAt.is_null()),
        )
        .order_by_asc(t_inspection_stations::Column::CreatedAt)
        .all(conn)
        .await
}

pub async fn page_inspection_stations(
    conn: &DatabaseConnection,
    mut page_index: u64,
    mut page_size: u64,
) -> Result<PageResult<t_inspection_stations::Model>, DbErr> {
    if page_index == 0 {
        page_index = 1;
    }
    if page_size == 0 {
        page_size = 10;
    }

    let page_find = TInspectionStations::find()
        .filter(t_inspection_stations::Column::DeletedAt.is_null())
        .order_by_desc(t_inspection_stations::Column::CreatedAt)
        .paginate(conn, page_size);

    let mut page = PageResult::<t_inspection_stations::Model> {
        records: vec![],
        page_index,
        page_size,
        total_count: 0,
        pages: 0,
    };

    let v = page_find.num_items_and_pages().await?;
    page.total_count = v.number_of_items;
    page.pages = v.number_of_pages;

    page.records = page_find.fetch_page(page_index - 1).await?;
    Ok(page)
}

pub async fn update_inspection_station(
    conn: &DatabaseConnection,
    id: Uuid,
    station: t_inspection_stations::ActiveModel,
) -> Result<Option<t_inspection_stations::Model>, DbErr> {
    let existing = find_inspection_station_by_id(conn, id).await?;
    if existing.is_none() {
        return Ok(None);
    }

    let mut station = station;
    station.id = ActiveValue::set(id);
    station.created_at = ActiveValue::set(existing.unwrap().created_at);

    TInspectionStations::update(station).exec(conn).await.map(Some)
}

/// Soft delete an inspection station by ID
pub async fn delete_inspection_station(
    conn: &DatabaseConnection,
    id: Uuid,
) -> Result<bool, DbErr> {
    let existing = find_inspection_station_by_id(conn, id).await?;
    if existing.is_none() {
        return Ok(false);
    }

    let mut station: t_inspection_stations::ActiveModel = existing.unwrap().into();
    station.deleted_at = ActiveValue::set(Some(chrono::Local::now().fixed_offset()));
    station.updated_at = ActiveValue::set(chrono::Local::now().fixed_offset());

    TInspectionStations::update(station).exec(conn).await?;
    Ok(true)
}

// ============================================================================
// Station ROI CRUD Operations
// ============================================================================

pub async fn insert_station_roi(
    conn: &DatabaseConnection,
    roi: t_station_rois::ActiveModel,
) -> Result<InsertResult<t_station_rois::ActiveModel>, DbErr> {
    TStationRois::insert(roi).exec(conn).await
}

pub async fn find_station_roi_by_id(
    conn: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<t_station_rois::Model>, DbErr> {
    TStationRois::find()
        .filter(
            t_station_rois::Column::Id
                .eq(id)
                .and(t_station_rois::Column::DeletedAt.is_null()),
        )
        .one(conn)
        .await
}

pub async fn find_station_rois_by_station_id(
    conn: &DatabaseConnection,
    station_id: Uuid,
) -> Result<Vec<t_station_rois::Model>, DbErr> {
    TStationRois::find()
        .filter(
            t_station_rois::Column::StationId
                .eq(station_id)
                .and(t_station_rois::Column::DeletedAt.is_null()),
        )
        .order_by_asc(t_station_rois::Column::CreatedAt)
        .all(conn)
        .await
}

pub async fn find_station_rois_by_purpose(
    conn: &DatabaseConnection,
    station_id: Uuid,
    purpose: t_station_rois::RoiPurpose,
) -> Result<Vec<t_station_rois::Model>, DbErr> {
    TStationRois::find()
        .filter(
            t_station_rois::Column::StationId
                .eq(station_id)
                .and(t_station_rois::Column::Purpose.eq(purpose))
                .and(t_station_rois::Column::Enabled.eq(true))
                .and(t_station_rois::Column::DeletedAt.is_null()),
        )
        .all(conn)
        .await
}

pub async fn find_enabled_station_rois(
    conn: &DatabaseConnection,
    station_id: Uuid,
) -> Result<Vec<t_station_rois::Model>, DbErr> {
    TStationRois::find()
        .filter(
            t_station_rois::Column::StationId
                .eq(station_id)
                .and(t_station_rois::Column::Enabled.eq(true))
                .and(t_station_rois::Column::DeletedAt.is_null()),
        )
        .all(conn)
        .await
}

pub async fn update_station_roi(
    conn: &DatabaseConnection,
    id: Uuid,
    roi: t_station_rois::ActiveModel,
) -> Result<Option<t_station_rois::Model>, DbErr> {
    let existing = find_station_roi_by_id(conn, id).await?;
    if existing.is_none() {
        return Ok(None);
    }

    let mut roi = roi;
    roi.id = ActiveValue::set(id);
    roi.created_at = ActiveValue::set(existing.unwrap().created_at);

    TStationRois::update(roi).exec(conn).await.map(Some)
}

/// Soft delete a station ROI by ID
pub async fn delete_station_roi(
    conn: &DatabaseConnection,
    id: Uuid,
) -> Result<bool, DbErr> {
    let existing = find_station_roi_by_id(conn, id).await?;
    if existing.is_none() {
        return Ok(false);
    }

    let mut roi: t_station_rois::ActiveModel = existing.unwrap().into();
    roi.deleted_at = ActiveValue::set(Some(chrono::Local::now().fixed_offset()));
    roi.updated_at = ActiveValue::set(chrono::Local::now().fixed_offset());

    TStationRois::update(roi).exec(conn).await?;
    Ok(true)
}

/// Delete all ROIs for a station (soft delete)
pub async fn delete_station_rois_by_station_id(
    conn: &DatabaseConnection,
    station_id: Uuid,
) -> Result<u64, DbErr> {
    let rois = find_station_rois_by_station_id(conn, station_id).await?;
    let count = rois.len() as u64;

    for roi in rois {
        let mut roi_model: t_station_rois::ActiveModel = roi.into();
        roi_model.deleted_at = ActiveValue::set(Some(chrono::Local::now().fixed_offset()));
        roi_model.updated_at = ActiveValue::set(chrono::Local::now().fixed_offset());
        TStationRois::update(roi_model).exec(conn).await?;
    }

    Ok(count)
}
