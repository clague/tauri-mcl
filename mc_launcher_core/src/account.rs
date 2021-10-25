use anyhow::{Result, bail};
use rand::Rng;
use serde::{Deserialize, Serialize, ser, de};
use serde_json::{json, Value as Json};
use reqwest::{Client as ReqwestClient, header::*};
use warp::{Filter, http::StatusCode, reply};
use chrono::{Utc};
use magic_crypt::{MagicCryptTrait, new_magic_crypt};
use tokio::sync::mpsc;

static MAGIC_KEY: &'static str = "1145141919810";

#[derive(Default, Deserialize, Serialize, Clone)]
pub struct AccountInfo {
    is_valid: bool, // have logined?
    is_default: bool, // default account?
    uuid: String,

    pub name: String,
    #[serde(serialize_with = "serialize_string_encrypted", deserialize_with = "deserialize_string_encrypted")]
    pub refresh_token: String, // Microsoft's refresh token
    #[serde(serialize_with = "serialize_string_encrypted", deserialize_with = "deserialize_string_encrypted")]
    pub access_token: String, // MC access token
    pub last_refresh_time: i64,
}

fn serialize_string_encrypted<S>(token: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    let crypt = new_magic_crypt!(MAGIC_KEY, 256);
    serializer.serialize_str(crypt.encrypt_str_to_base64(&token).as_str())
}

fn deserialize_string_encrypted<'de, D>(deserializer: D) -> Result<String, D::Error> 
where
    D: de::Deserializer<'de>,
{
    let original: String = de::Deserialize::deserialize(deserializer)?;
    let crypt = new_magic_crypt!(MAGIC_KEY, 256);
    match crypt.decrypt_base64_to_string(&original) {
        Ok(s) => Ok(s),
        Err(_) => Ok(original), 
    }
}

impl std::fmt::Display for AccountInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {

        match serde_json::to_string(self) {
            Ok(s) => write!(f, "{}", s),
            Err(e) => write!(f, "{}", e),
        }
    }
}

impl AccountInfo {
    const CLIENT_ID: &'static str = "ec20f5c7-5a39-4beb-8844-f0b8df3a0502";
    const REDIRECT_URI: &'static str = "http%3A%2F%2Flocalhost%3A7878%2Fapi%2Fauth%2Fredirect";
    const AUTHORIZATION_URL: &'static str = "https://login.live.com/oauth20_authorize.srf";
    const TOKEN_URL: &'static str = "https://login.live.com/oauth20_token.srf";
    const XBL_URL: &'static str = "https://user.auth.xboxlive.com/user/authenticate";
    const XSTS_URL: &'static str = "https://xsts.auth.xboxlive.com/xsts/authorize";
    const LOGIN_URL: &'static str = "https://api.minecraftservices.com/authentication/login_with_xbox";
    const CHECK_URL: &'static str = "https://api.minecraftservices.com/entitlements/mcstore";
    const PROFILE_URL: &'static str = "https://api.minecraftservices.com/minecraft/profile";

    pub fn set_refresh_token(&mut self, token: String) {
        self.refresh_token = token;
        self.last_refresh_time = Utc::now().timestamp();
    }

    //refresh token
    pub async fn refresh(&mut self)-> Result<()> {
        if self.is_valid {
            let request_body = format!("\
                client_id={}\
                &refresh_token={}\
                &grant_type=refresh_token\
                &redirect_uri={}", AccountInfo::CLIENT_ID, self.refresh_token,  AccountInfo::REDIRECT_URI);

            let reqwest_client = ReqwestClient::new();

            let received: Json = reqwest_client
                .post(AccountInfo::TOKEN_URL)
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(request_body.into_bytes())
                .send()
                .await?
                .json()
                .await?;

            let access_token = received["access_token"].as_str().unwrap_or(""); 
            let refresh_token = received["refresh_token"].as_str().unwrap_or("");
            
            self.set_refresh_token(refresh_token.to_string());
            self.last_refresh_time = Utc::now().timestamp();

            self.get_access_token(access_token.to_string()).await // Fetch MC access token
        }
        else {
            bail!("Not login yet")
        }
    }

    //login method for microsoft account
    pub async fn oauth2_login(&mut self) -> Result<()> {
        let state: String =rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();

        let reqwest_client = ReqwestClient::new();

        //Fetch token
        let auth_url = String::from(format!("{}?client_id={}\
            &response_type=code\
            &redirect_uri={}\
            &scope=Xboxlive.signin+Xboxlive.offline_access\
            &state={}", AccountInfo::AUTHORIZATION_URL, AccountInfo::CLIENT_ID, AccountInfo::REDIRECT_URI, state));

        println!("Browsing to: {}", auth_url);

        open::that(auth_url)?;

        let received = listen(7878).await?;

        if received.state != state {
            bail!("CSRF token mismatch :(");
        }

        //Fetch authorization token
        let request_body = format!("\
            client_id={}\
            &code={}\
            &grant_type=authorization_code\
            &redirect_uri={}", AccountInfo::CLIENT_ID, received.code, AccountInfo::REDIRECT_URI);
        
        let received: Json = reqwest_client
            .post(AccountInfo::TOKEN_URL)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(request_body.into_bytes())
            .send()
            .await?
            .json()
            .await?;

        let access_token = received["access_token"].as_str().unwrap_or("");
        let refresh_token = received["refresh_token"].as_str().unwrap_or("");

        self.set_refresh_token(refresh_token.to_string());

        self.get_access_token(access_token.to_string()).await?; // Fetch MC access token
        
        //Check game ownership
        let received: Json = reqwest_client
            .get(AccountInfo::CHECK_URL)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .send()
            .await?
            .json()
            .await?;

        if received["items"].as_array().unwrap().len() < 2 {
            bail!("You don't own Minecraft!!");
        }

        //Get profile
        let received: Json = reqwest_client
            .get(AccountInfo::PROFILE_URL)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .send()
            .await?
            .json()
            .await?;

        let uuid = received["id"].as_str().unwrap_or("");
        let name  = received["name"].as_str().unwrap_or("");

        self.uuid = uuid.to_string();
        self.name = name.to_string();
        self.is_valid = true;

        Ok(())
    }

    //Use Microsoft's token to get minecraft access token
    async fn get_access_token(&mut self, token: String) -> Result<()> {
        let reqwest_client = ReqwestClient::new();

        //Fetch Xbox token
        let json_payload = json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", token)
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        });

        let received: Json = reqwest_client
            .post(AccountInfo::XBL_URL)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .json(&json_payload)
            .send()
            .await?
            .json()
            .await?;

        let user_hash = received["DisplayClaims"]["xui"][0]["uhs"].as_str().unwrap_or("");
        let xbl_token = received["Token"].as_str().unwrap_or("");

        //Fetch XSTS token
        let json_payload = json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [
                    format!("{}", xbl_token)
                ]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        });

        let received: Json = reqwest_client
            .post(AccountInfo::XSTS_URL)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .json(&json_payload)
            .send()
            .await?
            .json()
            .await?;

        let xsts_token = received["Token"].as_str().unwrap_or("");

        //Fetch minecraft token
        let json_payload = json!({
            "identityToken": format!("XBL3.0 x={};{}", user_hash, xsts_token)
        });

        let received: Json = reqwest_client
            .post(AccountInfo::LOGIN_URL)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .json(&json_payload)
            .send()
            .await?
            .json()
            .await?;

        self.access_token = received["access_token"].as_str().unwrap_or("").to_string();

        Ok(())
    }

}


#[derive(Deserialize)]
struct ReceivedCode {
    pub code: String,
    pub state: String,
}

async fn listen(port: u16) -> Result<ReceivedCode> {
    let (tx, mut rx) = mpsc::channel::<ReceivedCode>(1);

    let route = warp::path!("api" / "auth" / "redirect")
        .and(warp::query::<ReceivedCode>())
        .and_then(move |r: ReceivedCode| {
                let tx = tx.clone();
                async move {
                    if r.code.len() > 0 && r.state.len() > 0 {
                        match tx.send(r).await {
                            Ok(()) => Ok(reply::with_status("Successfully received code, you can close the tab now.".to_string(), StatusCode::OK)),
                            Err(_) => Err(warp::reject::reject()),
                        }
                    }
                    else {
                        Err(warp::reject::reject())
                    }
                }
            }
        );

    let server = warp::serve(route).run(([127, 0, 0, 1], port));

    tokio::select! {
        _ = server => Err(anyhow::anyhow!("The server abnormally quit!")),
        r = rx.recv() => r.ok_or(anyhow::anyhow!("Can't receive code!")),
        _ = async {
            tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
        } => Err(anyhow::anyhow!("Wait for too much time")),
    }
}
