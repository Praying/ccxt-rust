use super::{parse_decimal, value_to_hashmap};
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{Balance, BalanceEntry},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde_json::Value;
use std::collections::HashMap;

/// Parse balance data from Binance account info.
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    if let Some(balances_array) = data["balances"].as_array() {
        for balance in balances_array {
            let currency = balance["asset"]
                .as_str()
                .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
                .to_string();

            let free = parse_decimal(balance, "free").unwrap_or(Decimal::ZERO);
            let locked = parse_decimal(balance, "locked").unwrap_or(Decimal::ZERO);
            let total = free + locked;

            balances.insert(
                currency,
                BalanceEntry {
                    free,
                    used: locked,
                    total,
                },
            );
        }
    }

    Ok(Balance {
        balances,
        info: value_to_hashmap(data),
    })
}

/// Parse balance information with account type support.
pub fn parse_balance_with_type(data: &Value, account_type: &str) -> Result<Balance> {
    let mut balances = HashMap::new();

    match account_type {
        "spot" => {
            if let Some(balances_array) = data["balances"].as_array() {
                for item in balances_array {
                    if let Some(asset) = item["asset"].as_str() {
                        let free = if let Some(free_str) = item["free"].as_str() {
                            free_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["free"].as_f64().unwrap_or(0.0)
                        };

                        let locked = if let Some(locked_str) = item["locked"].as_str() {
                            locked_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["locked"].as_f64().unwrap_or(0.0)
                        };

                        balances.insert(
                            asset.to_string(),
                            BalanceEntry {
                                free: Decimal::from_f64(free).unwrap_or(Decimal::ZERO),
                                used: Decimal::from_f64(locked).unwrap_or(Decimal::ZERO),
                                total: Decimal::from_f64(free + locked).unwrap_or(Decimal::ZERO),
                            },
                        );
                    }
                }
            }
        }
        "margin" | "cross" => {
            if let Some(user_assets) = data["userAssets"].as_array() {
                for item in user_assets {
                    if let Some(asset) = item["asset"].as_str() {
                        let free = if let Some(free_str) = item["free"].as_str() {
                            free_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["free"].as_f64().unwrap_or(0.0)
                        };

                        let locked = if let Some(locked_str) = item["locked"].as_str() {
                            locked_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["locked"].as_f64().unwrap_or(0.0)
                        };

                        balances.insert(
                            asset.to_string(),
                            BalanceEntry {
                                free: Decimal::from_f64(free).unwrap_or(Decimal::ZERO),
                                used: Decimal::from_f64(locked).unwrap_or(Decimal::ZERO),
                                total: Decimal::from_f64(free + locked).unwrap_or(Decimal::ZERO),
                            },
                        );
                    }
                }
            }
        }
        "isolated" => {
            if let Some(assets) = data["assets"].as_array() {
                for item in assets {
                    if let Some(base_asset) = item["baseAsset"].as_object() {
                        if let Some(asset) = base_asset["asset"].as_str() {
                            let free = if let Some(free_str) = base_asset["free"].as_str() {
                                free_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                base_asset["free"].as_f64().unwrap_or(0.0)
                            };

                            let locked = if let Some(locked_str) = base_asset["locked"].as_str() {
                                locked_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                base_asset["locked"].as_f64().unwrap_or(0.0)
                            };

                            balances.insert(
                                asset.to_string(),
                                BalanceEntry {
                                    free: Decimal::from_f64(free).unwrap_or(Decimal::ZERO),
                                    used: Decimal::from_f64(locked).unwrap_or(Decimal::ZERO),
                                    total: Decimal::from_f64(free + locked)
                                        .unwrap_or(Decimal::ZERO),
                                },
                            );
                        }
                    }

                    if let Some(quote_asset) = item["quoteAsset"].as_object() {
                        if let Some(asset) = quote_asset["asset"].as_str() {
                            let free = if let Some(free_str) = quote_asset["free"].as_str() {
                                free_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                quote_asset["free"].as_f64().unwrap_or(0.0)
                            };

                            let locked = if let Some(locked_str) = quote_asset["locked"].as_str() {
                                locked_str.parse::<f64>().unwrap_or(0.0)
                            } else {
                                quote_asset["locked"].as_f64().unwrap_or(0.0)
                            };

                            balances.insert(
                                asset.to_string(),
                                BalanceEntry {
                                    free: Decimal::from_f64(free).unwrap_or(Decimal::ZERO),
                                    used: Decimal::from_f64(locked).unwrap_or(Decimal::ZERO),
                                    total: Decimal::from_f64(free + locked)
                                        .unwrap_or(Decimal::ZERO),
                                },
                            );
                        }
                    }
                }
            }
        }
        "linear" | "future" | "inverse" | "delivery" => {
            let assets = if let Some(arr) = data.as_array() {
                arr.clone()
            } else if let Some(arr) = data["assets"].as_array() {
                arr.clone()
            } else {
                vec![]
            };

            for item in &assets {
                if let Some(asset) = item["asset"].as_str() {
                    let available_balance =
                        if let Some(balance_str) = item["availableBalance"].as_str() {
                            balance_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["availableBalance"].as_f64().unwrap_or(0.0)
                        };

                    let wallet_balance = if let Some(balance_str) = item["walletBalance"].as_str() {
                        balance_str.parse::<f64>().unwrap_or(0.0)
                    } else {
                        item["walletBalance"].as_f64().unwrap_or(0.0)
                    };

                    let wallet_balance = if wallet_balance == 0.0 {
                        if let Some(balance_str) = item["balance"].as_str() {
                            balance_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["balance"].as_f64().unwrap_or(wallet_balance)
                        }
                    } else {
                        wallet_balance
                    };

                    let used = wallet_balance - available_balance;

                    balances.insert(
                        asset.to_string(),
                        BalanceEntry {
                            free: Decimal::from_f64(available_balance).unwrap_or(Decimal::ZERO),
                            used: Decimal::from_f64(used).unwrap_or(Decimal::ZERO),
                            total: Decimal::from_f64(wallet_balance).unwrap_or(Decimal::ZERO),
                        },
                    );
                }
            }
        }
        "funding" => {
            if let Some(assets) = data.as_array() {
                for item in assets {
                    if let Some(asset) = item["asset"].as_str() {
                        let free = if let Some(free_str) = item["free"].as_str() {
                            free_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["free"].as_f64().unwrap_or(0.0)
                        };

                        let locked = if let Some(locked_str) = item["locked"].as_str() {
                            locked_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["locked"].as_f64().unwrap_or(0.0)
                        };

                        let total = free + locked;

                        balances.insert(
                            asset.to_string(),
                            BalanceEntry {
                                free: Decimal::from_f64(free).unwrap_or(Decimal::ZERO),
                                used: Decimal::from_f64(locked).unwrap_or(Decimal::ZERO),
                                total: Decimal::from_f64(total).unwrap_or(Decimal::ZERO),
                            },
                        );
                    }
                }
            }
        }
        "option" => {
            if let Some(asset) = data["asset"].as_str() {
                let equity = if let Some(equity_str) = data["equity"].as_str() {
                    equity_str.parse::<f64>().unwrap_or(0.0)
                } else {
                    data["equity"].as_f64().unwrap_or(0.0)
                };

                let available = if let Some(available_str) = data["available"].as_str() {
                    available_str.parse::<f64>().unwrap_or(0.0)
                } else {
                    data["available"].as_f64().unwrap_or(0.0)
                };

                let used = equity - available;

                balances.insert(
                    asset.to_string(),
                    BalanceEntry {
                        free: Decimal::from_f64(available).unwrap_or(Decimal::ZERO),
                        used: Decimal::from_f64(used).unwrap_or(Decimal::ZERO),
                        total: Decimal::from_f64(equity).unwrap_or(Decimal::ZERO),
                    },
                );
            }
        }
        "portfolio" => {
            let assets = if let Some(arr) = data.as_array() {
                arr.clone()
            } else if data.is_object() {
                vec![data.clone()]
            } else {
                vec![]
            };

            for item in &assets {
                if let Some(asset) = item["asset"].as_str() {
                    let total_wallet_balance =
                        if let Some(balance_str) = item["totalWalletBalance"].as_str() {
                            balance_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["totalWalletBalance"].as_f64().unwrap_or(0.0)
                        };

                    let available_balance =
                        if let Some(balance_str) = item["availableBalance"].as_str() {
                            balance_str.parse::<f64>().unwrap_or(0.0)
                        } else {
                            item["availableBalance"].as_f64().unwrap_or(0.0)
                        };

                    let free = if available_balance > 0.0 {
                        available_balance
                    } else if let Some(cross_str) = item["crossWalletBalance"].as_str() {
                        cross_str.parse::<f64>().unwrap_or(0.0)
                    } else {
                        item["crossWalletBalance"].as_f64().unwrap_or(0.0)
                    };

                    let used = total_wallet_balance - free;

                    balances.insert(
                        asset.to_string(),
                        BalanceEntry {
                            free: Decimal::from_f64(free).unwrap_or(Decimal::ZERO),
                            used: Decimal::from_f64(used).unwrap_or(Decimal::ZERO),
                            total: Decimal::from_f64(total_wallet_balance).unwrap_or(Decimal::ZERO),
                        },
                    );
                }
            }
        }
        _ => {
            return Err(Error::from(ParseError::invalid_value(
                "account_type",
                format!("Unsupported account type: {}", account_type),
            )));
        }
    }

    let mut info_map = HashMap::new();
    if let Some(obj) = data.as_object() {
        for (k, v) in obj {
            info_map.insert(k.clone(), v.clone());
        }
    }

    Ok(Balance {
        balances,
        info: info_map,
    })
}
