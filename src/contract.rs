#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, Uint128,
};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, DepositResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, Deposits, CONFIG, DEPOSITS};

/*
const CONTRACT_NAME: &str = "crates.io:deposit-native-example";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
 */

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let admin = msg.admin.unwrap_or_else(|| info.sender.to_string());
    let validated_admin = deps.api.addr_validate(&admin)?;
    let config = Config {
        admin: validated_admin.clone(),
        deposits_enabled: true,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", validated_admin.to_string())
        .add_attribute("deposits_enabled", "true"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {} => execute_deposit(deps, info),
        ExecuteMsg::Withdraw { amount, denom } => execute_withdraw(deps, info, amount, denom),
        ExecuteMsg::TransferOwnership { new_admin_address } => {
            execute_transfer_ownership(deps, info, new_admin_address)
        }
        ExecuteMsg::DisableDeposits {} => execute_disable_deposits(deps, info),
        ExecuteMsg::EnableDeposits {} => execute_enable_deposits(deps, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Deposits { address } => to_binary(&query_deposits(deps, address)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn execute_deposit(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.deposits_enabled == false {
        Err(ContractError::DepositsDisabled {})
    } else {
        let sender = info.sender.clone().into_string();
        let d_coins = info.funds[0].clone();
        //check to see if u
        match DEPOSITS.load(deps.storage, (&sender, d_coins.denom.as_str())) {
            Ok(mut deposit) => {
                //add coins to their account
                deposit.coins.amount = deposit.coins.amount.checked_add(d_coins.amount).unwrap();
                deposit.count = deposit.count.checked_add(1).unwrap();
                DEPOSITS
                    .save(deps.storage, (&sender, d_coins.denom.as_str()), &deposit)
                    .unwrap();
            }
            Err(_) => {
                //user does not exist, add them.
                let deposit = Deposits {
                    count: 1,
                    owner: info.sender,
                    coins: d_coins.clone(),
                };
                DEPOSITS
                    .save(deps.storage, (&sender, d_coins.denom.as_str()), &deposit)
                    .unwrap();
            }
        }
        Ok(Response::new()
            .add_attribute("execute", "deposit")
            .add_attribute("denom", d_coins.denom)
            .add_attribute("amount", d_coins.amount))
    }
}

pub fn execute_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    amount: u128,
    denom: String,
) -> Result<Response, ContractError> {
    let sender = info.sender.clone().into_string();

    let mut deposit = DEPOSITS
        .load(deps.storage, (&sender, denom.as_str()))
        .unwrap();
    deposit.coins.amount = deposit
        .coins
        .amount
        .checked_sub(Uint128::from(amount))
        .unwrap();
    deposit.count = deposit.count.checked_sub(1).unwrap();
    DEPOSITS
        .save(deps.storage, (&sender, denom.as_str()), &deposit)
        .unwrap();

    let msg = BankMsg::Send {
        to_address: sender.clone(),
        amount: vec![coin(amount, denom.clone())],
    };

    Ok(Response::new()
        .add_attribute("execute", "withdraw")
        .add_attribute("denom", denom)
        .add_attribute("amount", amount.to_string())
        .add_message(msg))
}

pub fn execute_transfer_ownership(
    deps: DepsMut,
    info: MessageInfo,
    new_admin_address: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let sender = info.sender.clone().into_string();

    if sender != config.admin {
        Err(ContractError::Unauthorized {})
    } else {
        let validated_address = deps.api.addr_validate(&new_admin_address).unwrap();
        config.admin = validated_address.clone();
        CONFIG.save(deps.storage, &config)?;
        Ok(Response::new()
            .add_attribute("execute", "transfer_ownership")
            .add_attribute("admin", validated_address.to_string()))
    }
}

pub fn execute_disable_deposits(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let sender = info.sender.clone().into_string();

    if sender != config.admin {
        Err(ContractError::Unauthorized {})
    } else {
        config.deposits_enabled = false;
        CONFIG.save(deps.storage, &config)?;
        Ok(Response::new()
            .add_attribute("execute", "disable_deposits")
            .add_attribute("deposits_enabled", "false"))
    }
}

pub fn execute_enable_deposits(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let sender = info.sender.clone().into_string();

    if sender != config.admin {
        Err(ContractError::Unauthorized {})
    } else {
        config.deposits_enabled = true;
        CONFIG.save(deps.storage, &config)?;
        Ok(Response::new()
            .add_attribute("execute", "enable_deposits")
            .add_attribute("deposits_enabled", "true"))
    }
}

fn query_deposits(deps: Deps, address: String) -> StdResult<DepositResponse> {
    let res: StdResult<Vec<_>> = DEPOSITS
        .prefix(&address)
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    let deposits = res?;
    Ok(DepositResponse { deposits })
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.may_load(deps.storage)?;
    Ok(ConfigResponse { config })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coin, from_binary};

    const SENDER: &str = "sender_address";
    const AMOUNT: u128 = 100000;
    const DENOM: &str = "utest";

    fn setup_contract(deps: DepsMut) {
        let msg = InstantiateMsg {
            admin: Some(SENDER.to_string()),
        };
        let info = mock_info(SENDER, &[]);
        let res = instantiate(deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "instantiate"),
                attr("admin", SENDER),
                attr("deposits_enabled", "true")
            ]
        )
    }

    fn deposit_coins(deps: DepsMut, is_valid_deposit: bool) {
        let msg = ExecuteMsg::Deposit {};
        let coins = vec![coin(AMOUNT, DENOM.to_string())];
        let info = mock_info(SENDER, &coins);
        let _env = mock_env();
        if is_valid_deposit == true {
            let res = execute(deps, _env.clone(), info, msg).unwrap();
            assert_eq!("deposit".to_string(), res.attributes[0].value);
            assert_eq!(DENOM.to_string(), res.attributes[1].value);
            assert_eq!(AMOUNT.to_string(), res.attributes[2].value);
        } else {
            let _res = execute(deps, _env, info, msg).unwrap_err();
        }
    }

    fn withdraw_coins(deps: DepsMut) {
        let msg = ExecuteMsg::Withdraw {
            amount: AMOUNT,
            denom: DENOM.to_string(),
        };
        let info = mock_info(SENDER, &vec![]);
        let res = execute(deps, mock_env(), info, msg).unwrap();
        assert_eq!("withdraw".to_string(), res.attributes[0].value);
        assert_eq!(DENOM.to_string(), res.attributes[1].value);
        assert_eq!(AMOUNT.to_string(), res.attributes[2].value);
    }

    fn query_coins(deps: Deps) {
        let msg: QueryMsg = QueryMsg::Deposits {
            address: SENDER.to_string(),
        };
        let res = query(deps, mock_env(), msg).unwrap();
        let query = from_binary::<DepositResponse>(&res).unwrap();
        assert_eq!(SENDER, query.deposits[0].1.owner);
        assert_eq!(DENOM, query.deposits[0].1.coins.denom);
        assert_eq!(
            AMOUNT.to_string(),
            query.deposits[0].1.coins.amount.to_string()
        );
        assert_eq!(1, query.deposits[0].1.count);
    }

    fn enable_deposits(deps: DepsMut) {
        let msg = ExecuteMsg::EnableDeposits {};
        let info = mock_info(SENDER, &vec![]);
        let res = execute(deps, mock_env(), info, msg).unwrap();
        assert_eq!("enable_deposits".to_string(), res.attributes[0].value);
        assert_eq!("true", res.attributes[1].value);
    }

    fn disable_deposits(deps: DepsMut) {
        let msg = ExecuteMsg::DisableDeposits {};
        let info = mock_info(SENDER, &vec![]);
        let res = execute(deps, mock_env(), info, msg).unwrap();
        assert_eq!("disable_deposits".to_string(), res.attributes[0].value);
        assert_eq!("false", res.attributes[1].value);
    }

    #[test]
    fn _0_instantiate() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
    }

    #[test]
    fn _1_deposit() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        deposit_coins(deps.as_mut(), true);
    }

    //Add code to query the deposits and check if they were properly stored
    #[test]
    fn _2_query_deposit() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        deposit_coins(deps.as_mut(), true);
        query_coins(deps.as_ref());
    }

    #[test]
    fn _1_deposit_then_withdraw() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        deposit_coins(deps.as_mut(), true);
        withdraw_coins(deps.as_mut())
    }

    #[test]
    fn _1_deposit_when_deposits_disabled() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        disable_deposits(deps.as_mut());
        deposit_coins(deps.as_mut(), false);
    }

    #[test]
    fn _1_disable_deposits_then_enable_deposits_then_deposit() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        disable_deposits(deps.as_mut());
        enable_deposits(deps.as_mut());
        deposit_coins(deps.as_mut(), true);
    }
}
