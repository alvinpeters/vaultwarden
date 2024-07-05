use crate::db::nondiesel::transaction::NondieselTransaction;

pub trait Model: Sized where {
    type Transaction: NondieselTransaction<Self>;
}

pub trait KeyValuePair: Model {

}

pub trait Table: Model {
    fn get() -> Self::Transaction;

    fn set() -> Self::Transaction;
}



