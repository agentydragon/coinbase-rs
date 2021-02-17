use super::error::CBError;
use crate::DateTime;
use bigdecimal::BigDecimal;
use hyper::client::{Client, HttpConnector};
use hyper::{Body, Request, Uri};
use hyper_tls::HttpsConnector;
use std::collections::HashMap;

pub struct Public {
    pub(crate) uri: String,
    client: Client<HttpsConnector<HttpConnector>>,
}

impl Public {
    pub(crate) const USER_AGENT: &'static str = concat!("coinbase-rs/", env!("CARGO_PKG_VERSION"));

    pub fn new(uri: &str) -> Self {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, Body>(https);
        let uri = uri.to_string();

        Self { uri, client }
    }

    pub(crate) async fn call_future<U>(&self, request: Request<Body>) -> Result<U, CBError>
    where
        for<'de> U: serde::Deserialize<'de>,
    {
        let response = self.client.request(request).await.map_err(CBError::Http)?;
        let bytes = hyper::body::to_bytes(response.into_body())
            .await
            .map_err(CBError::Http)?;
        let res: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            serde_json::from_slice(&bytes)
                .map(CBError::Coinbase)
                .unwrap_or_else(|_| {
                    let data = String::from_utf8(bytes.to_vec()).unwrap();
                    CBError::Serde { error: e, data }
                })
        })?;
        let data = serde_json::from_slice(res["data"].to_string().as_bytes()).map_err(|e| {
            let data = String::from_utf8(bytes.to_vec()).unwrap();
            CBError::Serde { error: e, data }
        })?;
        Ok(data)
    }

    async fn get_pub<U>(&self, uri: &str) -> Result<U, CBError>
    where
        U: Send + 'static,
        for<'de> U: serde::Deserialize<'de>,
    {
        self.call_future(self.request(uri)).await
    }

    fn request(&self, uri: &str) -> Request<Body> {
        let uri: Uri = (self.uri.to_string() + uri).parse().unwrap();

        Request::get(uri)
            .header("User-Agent", Self::USER_AGENT)
            .body(Body::empty())
            .unwrap()
    }

    ///
    /// **Get currencies**
    ///
    /// List known currencies. Currency codes will conform to the ISO 4217 standard where possible.
    /// Currencies which have or had no representation in ISO 4217 may use a custom code (e.g.
    /// BTC).
    ///
    /// https://developers.coinbase.com/api/v2#currencies
    ///
    pub async fn currencies(&self) -> Result<Vec<Currency>, CBError> {
        self.get_pub("/currencies").await
    }

    ///
    /// **Get exchange rates**
    ///
    /// Get current exchange rates. Default base currency is USD but it can be defined as any
    /// supported currency. Returned rates will define the exchange rate for one unit of the base
    /// currency.
    ///
    /// https://developers.coinbase.com/api/v2#exchange-rates
    ///
    pub async fn exchange_rates(&self) -> Result<ExchangeRates, CBError> {
        self.get_pub("/exchange-rates").await
    }

    pub async fn exchange_rates_with_base(
        &self,
        base_currency: &str,
    ) -> Result<ExchangeRates, CBError> {
        self.get_pub(&format!("/exchange-rates?currency={}", base_currency))
            .await
    }

    ///
    /// **Get buy price**
    ///
    /// Get the total price to buy one bitcoin or ether.
    ///
    /// https://developers.coinbase.com/api/v2#get-buy-price
    ///
    pub async fn buy_price(&self, currency_pair: &str) -> Result<CurrencyPrice, CBError> {
        self.get_pub(&format!("/currency_pair/{}/buy", currency_pair))
            .await
    }

    ///
    /// **Get sell price**
    ///
    /// Get the total price to sell one bitcoin or ether.
    ///
    /// https://developers.coinbase.com/api/v2#get-sell-price
    ///
    pub async fn sell_price(&self, currency_pair: &str) -> Result<CurrencyPrice, CBError> {
        self.get_pub(&format!("/currency_pair/{}/sell", currency_pair))
            .await
    }

    ///
    /// **Get spot price**
    ///
    /// Get the current market price for a currency pair. This is usually somewhere in between the
    /// buy and sell price.
    ///
    /// https://developers.coinbase.com/api/v2#get-spot-price
    ///
    pub async fn spot_price(
        &self,
        currency_pair: &str,
        _date: Option<chrono::NaiveDate>,
    ) -> Result<CurrencyPrice, CBError> {
        self.get_pub(&format!("/currency_pair/{}/spot", currency_pair))
            .await
    }

    ///
    /// **Get current time**
    ///
    /// Get the API server time.
    ///
    /// https://developers.coinbase.com/api/v2#time
    ///
    pub async fn current_time(&self) -> Result<DateTime, CBError> {
        self.get_pub("/current_time").await
        //.map(|c: Adapter<Result = Result<T, CBError>>| c.iso)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Order {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

#[derive(Deserialize, Debug)]
pub struct Response {
    pub pagination: Pagination,
    pub data: serde_json::Value,
}

#[derive(Deserialize, Debug)]
pub struct Pagination {
    pub ending_before: Option<DateTime>,
    pub starting_after: Option<DateTime>,
    pub previous_ending_before: Option<DateTime>,
    pub next_starting_after: Option<DateTime>,
    pub limit: usize,
    pub order: Order,
    pub previous_uri: String,
    pub next_uri: String,
}

#[derive(Deserialize, Debug)]
pub struct Currency {
    pub id: String,
    pub name: String,
    pub min_size: BigDecimal,
}

#[derive(Deserialize, Debug)]
pub struct ExchangeRates {
    pub currency: String,
    pub rates: HashMap<String, BigDecimal>,
}

#[derive(Deserialize, Debug)]
pub struct CurrencyPrice {
    pub amount: BigDecimal,
    pub currency: String,
}

#[derive(Deserialize, Debug)]
struct CurrentTime {
    iso: DateTime,
}

#[cfg(test)]
mod test {
    use bigdecimal::FromPrimitive;

    use super::*;

    #[test]
    fn test_currencies_deserialize() {
        let input = r#"
    [
    {
        "id": "AED",
        "name": "United Arab Emirates Dirham",
        "min_size": "0.01000000"
    },
    {
        "id": "AFN",
        "name": "Afghan Afghani",
        "min_size": "0.01000000"
    },
    {
        "id": "ALL",
        "name": "Albanian Lek",
        "min_size": "0.01000000"
    },
    {
        "id": "AMD",
        "name": "Armenian Dram",
        "min_size": "0.01000000"
    }
    ]"#;
        let currencies: Vec<Currency> = serde_json::from_slice(input.as_bytes()).unwrap();
        assert_eq!(currencies.len(), 4);
    }

    #[test]
    fn test_exchange_rates_deserialize() {
        let input = r#"
    {
    "currency": "BTC",
    "rates": {
        "AED": "36.73",
        "AFN": "589.50",
        "ALL": "1258.82",
        "AMD": "4769.49",
        "ANG": "17.88",
        "AOA": "1102.76",
        "ARS": "90.37",
        "AUD": "12.93",
        "AWG": "17.93",
        "AZN": "10.48",
        "BAM": "17.38"
    }
    }"#;
        let exchange_rates: ExchangeRates = serde_json::from_slice(input.as_bytes()).unwrap();
        assert_eq!(exchange_rates.currency, "BTC");
        assert_eq!(exchange_rates.rates.len(), 11);
    }

    #[test]
    fn test_currency_price_deserialize() {
        let input = r#"
    {
    "amount": "1010.25",
    "currency": "USD"
    }"#;
        let currency_price: CurrencyPrice = serde_json::from_slice(input.as_bytes()).unwrap();
        assert_eq!(
            currency_price.amount,
            BigDecimal::from_f32(1010.25).unwrap()
        );
        assert_eq!(currency_price.currency, "USD");
    }

    #[test]
    fn test_current_time_deserialize() {
        let input = r#"
    {
    "iso": "2015-06-23T18:02:51Z",
    "epoch": 1435082571
    }"#;
        let time: crate::DateTime = serde_json::from_slice(input.as_bytes())
            .map(|c: CurrentTime| c.iso)
            .unwrap();
        assert_eq!(1435082571, time.timestamp());
    }
}
