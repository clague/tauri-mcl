use std::net::TcpListener;

use anyhow::{Result, bail};
use rand::Rng;
use serde::{Deserialize, Serialize, ser, de};
use serde_json::{json, Value as Json};
use reqwest::{Client as ReqwestClient, header::*};
use warp::{Filter, http::Response};
use chrono::{Utc};
use magic_crypt::{MagicCryptTrait, new_magic_crypt};
use tokio::{sync::{broadcast, mpsc}};

static MAGIC_KEY: &'static str = "1145141919810";

#[derive(Default, Deserialize, Serialize, Clone)]
pub struct AccountInfo {
    is_valid: bool, // have logined?
    pub uuid: String,

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
    const REDIRECT_URI: &'static str = "http%3A%2F%2Flocalhost%3APORT%2Fapi%2Fauth%2Fredirect";
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

            // Fetch MC access token
            self.access_token = AccountInfo::get_access_token(access_token).await?;
            
            Ok(())
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

        let mut port_holder = None;
        let mut port = 0;
        for i in 7878..65535 {
            if let Ok(l) = TcpListener::bind(("127.0.0.1", i)) {
                port = l.local_addr()?.port();
                port_holder = Some(l);
                break;
            }
        }
        if port_holder.is_none() {
            bail!("No available port!")
        }
        let redirect_uri = AccountInfo::REDIRECT_URI.replace("PORT", &port.to_string());

        //Fetch token
        let auth_url = String::from(format!("{}?client_id={}\
            &response_type=code\
            &redirect_uri={}\
            &scope=Xboxlive.signin+Xboxlive.offline_access\
            &state={}",
            AccountInfo::AUTHORIZATION_URL,
            AccountInfo::CLIENT_ID,
            redirect_uri,
            state));

        println!("Browsing to: {}", auth_url);

        open::that(auth_url)?;

        let received = listen(port_holder.unwrap()).await?;

        if received.state != state {
            bail!("CSRF token mismatch :(");
        }

        //Fetch authorization token
        let request_body = format!("\
            client_id={}\
            &code={}\
            &grant_type=authorization_code\
            &redirect_uri={}", AccountInfo::CLIENT_ID, received.code, redirect_uri);
        
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
        self.access_token = AccountInfo::get_access_token(access_token).await?; // Fetch MC access token
        
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

        //println!("{}", self);
        Ok(())
    }

    //Use Microsoft's token to get minecraft access token
    async fn get_access_token(token: &str) -> Result<String> {
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

        Ok(received["access_token"].as_str().unwrap_or_default().to_string())
    }

}


#[derive(Deserialize, Clone, Debug)]
struct ReceivedCode {
    pub code: String,
    pub state: String,
}

async fn listen(port_holder: TcpListener) -> Result<ReceivedCode> {
    let (tx, mut rx) = mpsc::channel::<ReceivedCode>(2);

    let route = warp::query::<ReceivedCode>()
        .and(warp::header::<String>("accept-language"))
        .and_then(move |r: ReceivedCode, accept_lang: String| {
            let tx = tx.clone();
            async move {
                if r.code.is_empty() || r.state.is_empty() {
                    return Err(warp::reject());
                }
                
                let mut message = "";
                if !accept_lang.is_empty() {
                    let langs = accept_lang.split(",");
                    for lang in langs {
                        if lang.starts_with("zh_CN") {
                            message = "???????????????????????????????????????";
                            break;
                        }
                        else if lang.starts_with("zh") {
                            message = "???????????????????????????????????????";
                            break;
                        }
                        else if lang.starts_with("en") {
                            message = "You can close this tab now!";
                            break;
                        }
                    }
                }
                if message.is_empty() {
                    message = "You can close this tab now!"
                }
                if let Ok(_) = tx.send(r).await {
                    Ok(Response::builder()
                        .header(CONTENT_TYPE, "text/html; charset=UTF-8")
                        .header(CONNECTION, "close")
                        .body(format!("<h1>{}</h1>", message))
                    )
                }
                else {
                    Err(warp::reject())
                }
            }
        });

    let port = port_holder.local_addr()?.port();
    drop(port_holder);
    let server = warp::serve(route).bind(([127, 0, 0, 1], port));
        //.bind_with_graceful_shutdown(([127, 0, 0, 1], port), async { rx.recv().await; });

    tokio::select! {
        _ = server => Err(anyhow::anyhow!("Server went down unexpectedly!")),
        r = rx.recv() => r.ok_or(anyhow::anyhow!("Can't receive code!")),
        _ = async {
            tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
        } => Err(anyhow::anyhow!("Wait for too much time")),
    }
}
