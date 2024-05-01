use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Operation {
    CreateAccount(AccountCreationOp),
    TransferFunds(FundTransferOp),
    GetFunds(GetFundsOp),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountCreationOp {
    pub account_id: String,
    pub starting_balance: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FundTransferOp {
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetFundsOp {
    pub account_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub from_node: String,
    pub operation: Operation,
}

impl Request {
    pub fn new_get_funds_request(node_id: String, account_id: String) -> Request {
        Request {
            from_node: node_id,
            operation: Operation::GetFunds(GetFundsOp { account_id }),
        }
    }

    pub fn new_create_account_request(
        node_id: String,
        account_id: String,
        starting_balance: f64,
    ) -> Request {
        Request {
            from_node: node_id,
            operation: Operation::CreateAccount(AccountCreationOp {
                account_id,
                starting_balance,
            }),
        }
    }

    pub fn new_transfer_funds_request(
        node_id: String,
        from_account_id: String,
        to_account_id: String,
        amount: f64,
    ) -> Request {
        Request {
            from_node: node_id,
            operation: Operation::TransferFunds(FundTransferOp {
                from_account_id,
                to_account_id,
                amount,
            }),
        }
    }
}
