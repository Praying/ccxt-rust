#![allow(dead_code)]

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        DepositAddress, Transaction, TransactionFee, TransactionStatus, TransactionType, Transfer,
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;

/// Check if a currency is a fiat currency.
pub fn is_fiat_currency(currency: &str) -> bool {
    matches!(
        currency.to_uppercase().as_str(),
        "USD" | "EUR" | "GBP" | "JPY" | "CNY" | "KRW" | "AUD" | "CAD" | "CHF" | "HKD" | "SGD"
    )
}

/// Extract internal transfer ID from transaction ID.
pub fn extract_internal_transfer_id(txid: &str) -> String {
    const PREFIX: &str = "Internal transfer ";
    txid.strip_prefix(PREFIX)
        .map_or_else(|| txid.to_string(), ToString::to_string)
}

/// Parse transaction status based on transaction type.
pub fn parse_transaction_status_by_type(
    status_value: &Value,
    is_deposit: bool,
) -> ccxt_core::types::TransactionStatus {
    if let Some(status_int) = status_value.as_i64() {
        if is_deposit {
            match status_int {
                1 | 6 => TransactionStatus::Ok,
                _ => TransactionStatus::Pending,
            }
        } else {
            match status_int {
                1 => TransactionStatus::Canceled,
                3 | 5 => TransactionStatus::Failed,
                6 => TransactionStatus::Ok,
                _ => TransactionStatus::Pending,
            }
        }
    } else if let Some(status_str) = status_value.as_str() {
        match status_str {
            "Failed" | "Refund Failed" => TransactionStatus::Failed,
            "Successful" => TransactionStatus::Ok,
            "Refunding" | "Refunded" => TransactionStatus::Canceled,
            _ => TransactionStatus::Pending,
        }
    } else {
        TransactionStatus::Pending
    }
}

/// Parse transaction (deposit or withdrawal) from Binance API response.
pub fn parse_transaction(
    data: &Value,
    transaction_type: ccxt_core::types::TransactionType,
) -> Result<ccxt_core::types::Transaction> {
    let is_deposit = matches!(transaction_type, TransactionType::Deposit);

    let id = if is_deposit {
        data["id"]
            .as_str()
            .or_else(|| data["orderNo"].as_str())
            .unwrap_or("")
            .to_string()
    } else {
        data["id"]
            .as_str()
            .or_else(|| data["withdrawOrderId"].as_str())
            .unwrap_or("")
            .to_string()
    };

    let currency = data["coin"]
        .as_str()
        .or_else(|| data["fiatCurrency"].as_str())
        .unwrap_or("")
        .to_string();

    let amount = data["amount"]
        .as_str()
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or(Decimal::ZERO);

    let fee = if is_deposit {
        None
    } else {
        data["transactionFee"]
            .as_str()
            .or_else(|| data["totalFee"].as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .map(|cost| TransactionFee {
                currency: currency.clone(),
                cost,
            })
    };

    let timestamp = if is_deposit {
        data["insertTime"].as_i64()
    } else {
        data["createTime"].as_i64().or_else(|| {
            data["applyTime"].as_str().and_then(|s| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .map(|dt| dt.and_utc().timestamp_millis())
            })
        })
    };

    let datetime = timestamp.and_then(|ts| ccxt_core::time::iso8601(ts).ok());

    let network = data["network"].as_str().map(ToString::to_string);

    let address = data["address"]
        .as_str()
        .or_else(|| data["depositAddress"].as_str())
        .map(ToString::to_string);

    let tag = data["addressTag"]
        .as_str()
        .or_else(|| data["tag"].as_str())
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);

    let mut txid = data["txId"]
        .as_str()
        .or_else(|| data["hash"].as_str())
        .map(ToString::to_string);

    let transfer_type = data["transferType"].as_i64();
    let is_internal = transfer_type == Some(1);

    if is_internal {
        if let Some(ref tx) = txid {
            txid = Some(extract_internal_transfer_id(tx));
        }
    }

    let status = if let Some(status_value) = data.get("status") {
        parse_transaction_status_by_type(status_value, is_deposit)
    } else {
        TransactionStatus::Pending
    };

    let updated = data["updateTime"].as_i64();

    let comment = data["info"]
        .as_str()
        .or_else(|| data["comment"].as_str())
        .map(ToString::to_string);

    Ok(Transaction {
        info: Some(data.clone()),
        id,
        txid,
        timestamp,
        datetime,
        network,
        address: address.clone(),
        address_to: if is_deposit { address.clone() } else { None },
        address_from: if is_deposit { None } else { address },
        tag: tag.clone(),
        tag_to: if is_deposit { tag.clone() } else { None },
        tag_from: if is_deposit { None } else { tag },
        transaction_type,
        amount,
        currency,
        status,
        updated,
        internal: Some(is_internal),
        comment,
        fee,
    })
}

/// Parse transfer record data from Binance universal transfer API.
pub fn parse_transfer(data: &Value) -> Result<Transfer> {
    let id = data["tranId"]
        .as_i64()
        .or_else(|| data["txId"].as_i64())
        .or_else(|| data["transactionId"].as_i64())
        .map(|id| id.to_string());

    let timestamp = data["timestamp"]
        .as_i64()
        .or_else(|| data["transactionTime"].as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    let currency = data["asset"]
        .as_str()
        .or_else(|| data["currency"].as_str())
        .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
        .to_string();

    let amount = if let Some(amount_str) = data["amount"].as_str() {
        amount_str.parse::<f64>().unwrap_or(0.0)
    } else {
        data["amount"].as_f64().unwrap_or(0.0)
    };

    let mut from_account = data["fromAccountType"].as_str().map(ToString::to_string);

    let mut to_account = data["toAccountType"].as_str().map(ToString::to_string);

    if from_account.is_none() || to_account.is_none() {
        if let Some(type_str) = data["type"].as_str() {
            let parts: Vec<&str> = type_str.split('_').collect();
            if parts.len() == 2 {
                from_account = Some(parts[0].to_lowercase());
                to_account = Some(parts[1].to_lowercase());
            }
        }
    }

    let status = data["status"].as_str().unwrap_or("SUCCESS").to_lowercase();

    Ok(Transfer {
        id,
        timestamp,
        datetime,
        currency,
        amount,
        from_account,
        to_account,
        status,
        info: Some(data.clone()),
    })
}

/// Parse deposit address from Binance API response.
pub fn parse_deposit_address(data: &Value) -> Result<ccxt_core::types::DepositAddress> {
    let currency = data["coin"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("coin")))?
        .to_string();

    let address = data["address"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("address")))?
        .to_string();

    let network = data["network"]
        .as_str()
        .map(ToString::to_string)
        .or_else(|| {
            data["url"].as_str().and_then(|url| {
                if url.contains("btc.com") {
                    Some("BTC".to_string())
                } else if url.contains("etherscan.io") {
                    Some("ETH".to_string())
                } else if url.contains("tronscan.org") {
                    Some("TRX".to_string())
                } else {
                    None
                }
            })
        });

    let tag = data["tag"]
        .as_str()
        .or_else(|| data["addressTag"].as_str())
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);

    Ok(DepositAddress {
        info: Some(data.clone()),
        currency,
        network,
        address,
        tag,
    })
}

/// Parse deposit and withdrawal fee information from Binance API.
pub fn parse_deposit_withdraw_fee(data: &Value) -> Result<ccxt_core::types::DepositWithdrawFee> {
    let currency = data["coin"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("coin")))?
        .to_string();

    let mut networks = Vec::new();
    let mut withdraw_fee = 0.0;
    let mut withdraw_min = 0.0;
    let mut withdraw_max = 0.0;
    let mut deposit_enable = false;
    let mut withdraw_enable = false;

    if let Some(network_list) = data["networkList"].as_array() {
        for network_data in network_list {
            let network = parse_network_info(network_data)?;

            if network_data["isDefault"].as_bool().unwrap_or(false) {
                withdraw_fee = network.withdraw_fee;
                withdraw_min = network.withdraw_min;
                withdraw_max = network.withdraw_max;
                deposit_enable = network.deposit_enable;
                withdraw_enable = network.withdraw_enable;
            }

            networks.push(network);
        }
    }

    if !networks.is_empty() && withdraw_fee == 0.0 {
        let first = &networks[0];
        withdraw_fee = first.withdraw_fee;
        withdraw_min = first.withdraw_min;
        withdraw_max = first.withdraw_max;
        deposit_enable = first.deposit_enable;
        withdraw_enable = first.withdraw_enable;
    }

    Ok(ccxt_core::types::DepositWithdrawFee {
        currency,
        withdraw_fee,
        withdraw_min,
        withdraw_max,
        deposit_enable,
        withdraw_enable,
        networks,
        info: Some(data.clone()),
    })
}

/// Parse network information from Binance API.
pub fn parse_network_info(data: &Value) -> Result<ccxt_core::types::NetworkInfo> {
    let network = data["network"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("network")))?
        .to_string();

    let name = data["name"].as_str().unwrap_or(&network).to_string();

    let withdraw_fee = data["withdrawFee"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["withdrawFee"].as_f64())
        .unwrap_or(0.0);

    let withdraw_min = data["withdrawMin"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["withdrawMin"].as_f64())
        .unwrap_or(0.0);

    let withdraw_max = data["withdrawMax"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["withdrawMax"].as_f64())
        .unwrap_or(0.0);

    let deposit_enable = data["depositEnable"].as_bool().unwrap_or(false);
    let withdraw_enable = data["withdrawEnable"].as_bool().unwrap_or(false);
    let deposit_confirmations = data["minConfirm"].as_u64().map(|v| v as u32);
    let withdraw_confirmations = data["unlockConfirm"].as_u64().map(|v| v as u32);

    Ok(ccxt_core::types::NetworkInfo {
        network,
        name,
        withdraw_fee,
        withdraw_min,
        withdraw_max,
        deposit_enable,
        withdraw_enable,
        deposit_confirmations,
        withdraw_confirmations,
    })
}

/// Parse multiple deposit and withdrawal fee information entries from Binance API.
pub fn parse_deposit_withdraw_fees(
    data: &Value,
) -> Result<Vec<ccxt_core::types::DepositWithdrawFee>> {
    if let Some(array) = data.as_array() {
        array
            .iter()
            .map(|item| parse_deposit_withdraw_fee(item))
            .collect()
    } else {
        Ok(vec![parse_deposit_withdraw_fee(data)?])
    }
}

/// Parse futures transfer type code from Binance API.
pub fn parse_futures_transfer_type(transfer_type: i32) -> Result<(&'static str, &'static str)> {
    match transfer_type {
        1 => Ok(("spot", "future")),
        2 => Ok(("future", "spot")),
        3 => Ok(("spot", "delivery")),
        4 => Ok(("delivery", "spot")),
        _ => Err(Error::invalid_request(format!(
            "Invalid futures transfer type: {}. Must be between 1 and 4",
            transfer_type
        ))),
    }
}
