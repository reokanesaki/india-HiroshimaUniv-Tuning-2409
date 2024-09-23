use crate::domains::auth_service::AuthService;
use crate::domains::dto::auth::{LoginRequestDto, LogoutRequestDto, RegisterRequestDto};
use crate::errors::AppError;
use crate::repositories::auth_repository::AuthRepositoryImpl;

//処理を追加
//use futures::future::Future;


//actix_webは、rustの非同期？フレームワーク
use actix_web::{web, HttpResponse};
//serdeは、構造体,列挙型とjson,ymlに変換するフレームワーク
use serde::{Deserialize, Serialize};

//deriveマクロの、Debugトレイト
//Deserializeは、上のserdeのトレイト
//structで構造体を定義
#[derive(Deserialize, Debug)]
pub struct ValidateSessionQueryParams {
    session_token: Option<String>,
}   

#[derive(Serialize, Debug)]
pub struct ValidationResponse {
    is_valid: bool,
}

//非同期の関数validate_session_handlerを宣言
pub async fn validate_session_handler(
    //web::Data<T>はactix-webの機能
    //<AuthService<AuthRepositoryImpl>>ジェネリック型を連続して使用
    service: web::Data<AuthService<AuthRepositoryImpl>>,
    query: web::Query<ValidateSessionQueryParams>,
    //以下のResultが成功したらHttpResponseを返す
    //失敗したらAppErrorを返す
) -> Result<HttpResponse, AppError> {
    //query.session_tokenがあれば(matchすれば)Someを実行
    //無ければNoneを実行
    //&query.session_tokenの&は、参照
    match &query.session_token {
        //Some()は、Rustのオプション型
        Some(session_token) => match service.validate_session(session_token.as_str()).await {
            Ok(is_valid) => Ok(HttpResponse::Ok().json(ValidationResponse { is_valid })),
            Err(_) => Ok(HttpResponse::Ok().json(ValidationResponse { is_valid: false })),
        },
        None => Ok(HttpResponse::Ok().json(ValidationResponse { is_valid: false })),
    }
}

//userを登録
pub async fn register_handler(
    service: web::Data<AuthService<AuthRepositoryImpl>>,
    req: web::Json<RegisterRequestDto>,
    //->は、Result型の戻り値を返す
) -> Result<HttpResponse, AppError> {
    match service
        .register_user(&req.username, &req.password, &req.role, req.area_id)
        //非同期処理で.registerの終了を待つ。
        .await
    {
        Ok(response) => Ok(HttpResponse::Created().json(response)),
        Err(err) => Err(err),
    }
}


pub async fn login_handler(
    service: web::Data<AuthService<AuthRepositoryImpl>>,
    req: web::Json<LoginRequestDto>,
) -> Result<HttpResponse, AppError> {
    match service.login_user(&req.username, &req.password).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(err) => Err(err),
    }
}

pub async fn logout_handler(
    service: web::Data<AuthService<AuthRepositoryImpl>>,
    req: web::Json<LogoutRequestDto>,
) -> Result<HttpResponse, AppError> {
    match service.logout_user(&req.session_token).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(_) => Ok(HttpResponse::Ok().finish()),
    }
}


//上の処理を改良


/* 
// 共通のハンドラー関数
async fn handle_service_call<F, T>(
    service_call: F,
) -> Result<HttpResponse, AppError>
where
    F: Future<Output = Result<T, AppError>>,
    T: serde::Serialize,  // JSONで返せるようにシリアライズを強制
{
    match service_call.await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(err) => Err(err),
    }
}

// ログインハンドラー
pub async fn login_handler(
    service: web::Data<AuthService<AuthRepositoryImpl>>,
    req: web::Json<LoginRequestDto>,
) -> Result<HttpResponse, AppError> {
    handle_service_call(service.login_user(&req.username, &req.password)).await
}

// ログアウトハンドラー
pub async fn logout_handler(
    service: web::Data<AuthService<AuthRepositoryImpl>>,
    req: web::Json<LogoutRequestDto>,
) -> Result<HttpResponse, AppError> {
    match service.logout_user(&req.session_token).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(_) => Ok(HttpResponse::Ok().finish()), // エラーを握りつぶして常にOKを返す場合
    }
}


//変更した処理終了
*/

#[derive(Deserialize, Debug)]
pub struct UserProfileImageQueryParams {
    w: Option<i32>,
    h: Option<i32>,
}

pub async fn user_profile_image_handler(
    service: web::Data<AuthService<AuthRepositoryImpl>>,
    path: web::Path<i32>,
    query: web::Query<UserProfileImageQueryParams>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let width = query.w.unwrap_or(500);
    let height = query.h.unwrap_or(500);
    let profile_image_byte = service
        .get_resized_profile_image_byte(user_id, width, height)
        .await?;
    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .body(profile_image_byte))
}
