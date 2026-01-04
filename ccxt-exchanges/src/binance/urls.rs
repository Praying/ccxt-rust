//! Binance API URLs for different environments and market types.

/// Binance API URLs.
#[derive(Debug, Clone)]
pub struct BinanceUrls {
    /// Public API URL.
    pub public: String,
    /// Private API URL.
    pub private: String,
    /// SAPI URL (Spot API).
    pub sapi: String,
    /// SAPI V2 URL.
    pub sapi_v2: String,
    /// FAPI URL (Futures API - short form).
    pub fapi: String,
    /// FAPI URL (Futures API).
    pub fapi_public: String,
    /// FAPI Private URL.
    pub fapi_private: String,
    /// DAPI URL (Delivery API - short form).
    pub dapi: String,
    /// DAPI URL (Delivery API).
    pub dapi_public: String,
    /// DAPI Private URL.
    pub dapi_private: String,
    /// EAPI URL (Options API - short form).
    pub eapi: String,
    /// EAPI URL (Options API).
    pub eapi_public: String,
    /// EAPI Private URL.
    pub eapi_private: String,
    /// PAPI URL (Portfolio Margin API).
    pub papi: String,
    /// WebSocket URL (Spot).
    pub ws: String,
    /// WebSocket Futures URL (USDT-margined perpetuals/futures).
    pub ws_fapi: String,
    /// WebSocket Delivery URL (Coin-margined perpetuals/futures).
    pub ws_dapi: String,
    /// WebSocket Options URL.
    pub ws_eapi: String,
}

impl BinanceUrls {
    /// Returns production environment URLs.
    pub fn production() -> Self {
        Self {
            public: "https://api.binance.com/api/v3".to_string(),
            private: "https://api.binance.com/api/v3".to_string(),
            sapi: "https://api.binance.com/sapi/v1".to_string(),
            sapi_v2: "https://api.binance.com/sapi/v2".to_string(),
            fapi: "https://fapi.binance.com/fapi/v1".to_string(),
            fapi_public: "https://fapi.binance.com/fapi/v1".to_string(),
            fapi_private: "https://fapi.binance.com/fapi/v1".to_string(),
            dapi: "https://dapi.binance.com/dapi/v1".to_string(),
            dapi_public: "https://dapi.binance.com/dapi/v1".to_string(),
            dapi_private: "https://dapi.binance.com/dapi/v1".to_string(),
            eapi: "https://eapi.binance.com/eapi/v1".to_string(),
            eapi_public: "https://eapi.binance.com/eapi/v1".to_string(),
            eapi_private: "https://eapi.binance.com/eapi/v1".to_string(),
            papi: "https://papi.binance.com/papi/v1".to_string(),
            ws: "wss://stream.binance.com:9443/ws".to_string(),
            ws_fapi: "wss://fstream.binance.com/ws".to_string(),
            ws_dapi: "wss://dstream.binance.com/ws".to_string(),
            ws_eapi: "wss://nbstream.binance.com/eoptions/ws".to_string(),
        }
    }

    /// Returns testnet URLs.
    pub fn testnet() -> Self {
        Self {
            public: "https://testnet.binance.vision/api/v3".to_string(),
            private: "https://testnet.binance.vision/api/v3".to_string(),
            sapi: "https://testnet.binance.vision/sapi/v1".to_string(),
            sapi_v2: "https://testnet.binance.vision/sapi/v2".to_string(),
            fapi: "https://testnet.binancefuture.com/fapi/v1".to_string(),
            fapi_public: "https://testnet.binancefuture.com/fapi/v1".to_string(),
            fapi_private: "https://testnet.binancefuture.com/fapi/v1".to_string(),
            dapi: "https://testnet.binancefuture.com/dapi/v1".to_string(),
            dapi_public: "https://testnet.binancefuture.com/dapi/v1".to_string(),
            dapi_private: "https://testnet.binancefuture.com/dapi/v1".to_string(),
            eapi: "https://testnet.binanceops.com/eapi/v1".to_string(),
            eapi_public: "https://testnet.binanceops.com/eapi/v1".to_string(),
            eapi_private: "https://testnet.binanceops.com/eapi/v1".to_string(),
            papi: "https://testnet.binance.vision/papi/v1".to_string(),
            ws: "wss://testnet.binance.vision/ws".to_string(),
            ws_fapi: "wss://stream.binancefuture.com/ws".to_string(),
            ws_dapi: "wss://dstream.binancefuture.com/ws".to_string(),
            ws_eapi: "wss://testnet.binanceops.com/ws-api/v3".to_string(),
        }
    }

    /// Returns demo environment URLs.
    pub fn demo() -> Self {
        Self {
            public: "https://demo-api.binance.com/api/v3".to_string(),
            private: "https://demo-api.binance.com/api/v3".to_string(),
            sapi: "https://demo-api.binance.com/sapi/v1".to_string(),
            sapi_v2: "https://demo-api.binance.com/sapi/v2".to_string(),
            fapi: "https://demo-fapi.binance.com/fapi/v1".to_string(),
            fapi_public: "https://demo-fapi.binance.com/fapi/v1".to_string(),
            fapi_private: "https://demo-fapi.binance.com/fapi/v1".to_string(),
            dapi: "https://demo-dapi.binance.com/dapi/v1".to_string(),
            dapi_public: "https://demo-dapi.binance.com/dapi/v1".to_string(),
            dapi_private: "https://demo-dapi.binance.com/dapi/v1".to_string(),
            eapi: "https://demo-eapi.binance.com/eapi/v1".to_string(),
            eapi_public: "https://demo-eapi.binance.com/eapi/v1".to_string(),
            eapi_private: "https://demo-eapi.binance.com/eapi/v1".to_string(),
            papi: "https://demo-papi.binance.com/papi/v1".to_string(),
            ws: "wss://demo-stream.binance.com/ws".to_string(),
            ws_fapi: "wss://demo-fstream.binance.com/ws".to_string(),
            ws_dapi: "wss://demo-dstream.binance.com/ws".to_string(),
            ws_eapi: "wss://demo-nbstream.binance.com/eoptions/ws".to_string(),
        }
    }
}
