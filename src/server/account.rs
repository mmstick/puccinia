use diesel::prelude::*;
use puccinia::database::ConnectionMutex;
use puccinia::database::models::{Wallet, Account, Position, Transaction};
use puccinia::database::schema::{wallets, accounts, positions, transactions};
use rocket::State;
use rocket_contrib::Template;
use rust_decimal::Decimal;
use std::str::FromStr;

#[get("/account/<wallet_id>/<id>")]
pub fn account(connection_mutex: State<ConnectionMutex>, wallet_id: String, id: String) -> Result<Template, String> {
    let connection = connection_mutex.lock().map_err(|err| format!("{}", err))?;

    #[derive(Serialize)]
    struct TransactionContext {
        transaction: Transaction,
        total: Decimal,
    }

    #[derive(Serialize)]
    struct Context {
        wallet: Wallet,
        account: Account,
        total: Decimal,
        positions: Vec<Position>,
        transactions: Vec<TransactionContext>,
    }

    let wallet = wallets::table
        .find(&wallet_id)
        .first::<Wallet>(&*connection)
        .map_err(|err| format!("{}", err))?;

    let account = accounts::table
        .find((&wallet_id, &id))
        .first::<Account>(&*connection)
        .map_err(|err| format!("{}", err))?;

    let mut context = Context {
        wallet: wallet,
        account: account,
        total: Decimal::new(0, 0),
        positions: Vec::new(),
        transactions: Vec::new()
    };

    let positions = positions::table
        .filter(positions::wallet_id.eq(&wallet_id))
        .filter(positions::account_id.eq(&id))
        .order(positions::id.asc())
        .load::<Position>(&*connection)
        .map_err(|err| format!("{}", err))?;

    for position in positions {
        if let Ok(value) = Decimal::from_str(&position.value) {
            context.total += value;
        }
        context.positions.push(position);
    }

    let transactions = transactions::table
        .filter(transactions::wallet_id.eq(&wallet_id))
        .filter(transactions::account_id.eq(&id))
        .order((transactions::time.desc(), transactions::id.desc()))
        .load::<Transaction>(&*connection)
        .map_err(|err| format!("{}", err))?;

    let mut current = context.total;
    for transaction in transactions {
        let mut next = current;
        if let Ok(value) = Decimal::from_str(&transaction.amount) {
            next -= value;
        }
        context.transactions.push(TransactionContext {
            transaction: transaction,
            total: current,
        });
        current = next;
    }

    Ok(Template::render("account", &context))
}
