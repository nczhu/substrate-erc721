use parity_codec::Encode;
use srml_support::{StorageMap, dispatch::Result};
use system::ensure_signed;
use runtime_primitives::traits::{Hash, Zero};
use rstd::prelude::*;

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash
    {
        Transfer(Option<AccountId>, Option<AccountId>, Hash),
        Approval(AccountId, AccountId, Hash),
        ApprovalForAll(AccountId, AccountId, bool),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as ERC721Storage {
        OwnedTokensCount get(balance_of): map T::AccountId => u32;
        TokenOwner get(owner_of): map T::Hash => Option<T::AccountId>;
        TokenApprovals get(get_approved): map T::Hash => Option<T::AccountId>;
        OperatorApprovals get(is_approved_for_all): map (T::AccountId, T::AccountId) => bool;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

    fn deposit_event<T>() = default;

    fn approve(origin, to: T::AccountId, token_id: T::Hash) -> Result {
        let sender = ensure_signed(origin)?;
        let owner = match Self::owner_of(token_id) {
            Some(c) => c,
            None => return Err("No owner for this token"),
        };

        ensure!(to != owner, "Owner is implicitly approved");
        ensure!(sender == owner || Self::is_approved_for_all((owner.clone(), sender.clone())), "You are not allowed to approve for this token");

        <TokenApprovals<T>>::insert(&token_id, &to);

        Self::deposit_event(RawEvent::Approval(owner, to, token_id));

        Ok(())
    }

    fn set_approval_for_all(origin, to: T::AccountId, approved: bool) -> Result {
        let sender = ensure_signed(origin)?;
        ensure!(to != sender, "You are already implicity approved for your own actions");
        <OperatorApprovals<T>>::insert((sender.clone(), to.clone()), approved);

        Self::deposit_event(RawEvent::ApprovalForAll(sender, to, approved));

        Ok(())
    }

    fn transfer_from(origin, from: T::AccountId, to: T::AccountId, token_id: T::Hash) -> Result {
        let sender = ensure_signed(origin)?;
        ensure!(Self::_is_approved_or_owner(sender, token_id), "You do not own this token");

        Self::_transfer_from(from, to, token_id)?;

        Ok(())
    }

    fn safe_transfer_from(origin, from: T::AccountId, to: T::AccountId, token_id: T::Hash) -> Result {
        let to_balance = <balances::Module<T>>::free_balance(&to);
        ensure!(!to_balance.is_zero(), "'to' account does not satisfy the `ExistentialDeposit` requirement");

        Self::transfer_from(origin, from, to, token_id)?;

        Ok(())
    }

    // Not part of ERC721, but allows you to play with the runtime
    fn create_token(origin) -> Result{
        let sender = ensure_signed(origin)?;
        let random_hash = (<system::Module<T>>::random_seed(), &sender).using_encoded(<T as system::Trait>::Hashing::hash);
        
        Self::_mint(sender, random_hash)?;

        Ok(())
    }
  }
}

impl<T: Trait> Module<T> {
    fn _exists(token_id: T::Hash) -> bool {
        return <TokenOwner<T>>::exists(token_id);
    }

    fn _is_approved_or_owner(spender: T::AccountId, token_id: T::Hash) -> bool {
        let owner = Self::owner_of(token_id);
        let approved_user = Self::get_approved(token_id);

        let approved_as_owner = match owner.clone() {
            Some(o) => o == spender,
            None => false,
        };

        let approved_as_delegate = match owner {
            Some(d) => Self::is_approved_for_all((d, spender.clone())),
            None => false,
        };

        let approved_as_user = match approved_user {
            Some(u) => u == spender,
            None => false,
        };

        return approved_as_owner || approved_as_user || approved_as_delegate
    }

    fn _mint(to: T::AccountId, token_id: T::Hash) -> Result {
        ensure!(!Self::_exists(token_id), "Token already exists");

        let balance_of = Self::balance_of(&to);

        let new_balance_of = match balance_of.checked_add(1) {
            Some(c) => c,
            None => return Err("Overflow adding a new token to account balance"),
        };

        <TokenOwner<T>>::insert(token_id, &to);
        <OwnedTokensCount<T>>::insert(&to, new_balance_of);

        Self::deposit_event(RawEvent::Transfer(None, Some(to), token_id));

        Ok(())
    }

    fn _burn(token_id: T::Hash) -> Result {
        let owner = match Self::owner_of(token_id) {
            Some(c) => c,
            None => return Err("No owner for this token"),
        };

        let balance_of = Self::balance_of(&owner);

        let new_balance_of = match balance_of.checked_sub(1) {
            Some(c) => c,
            None => return Err("Underflow subtracting a token to account balance"),
        };

        Self::_clear_approval(token_id)?;

        <OwnedTokensCount<T>>::insert(&owner, new_balance_of);
        <TokenOwner<T>>::remove(token_id);

        Self::deposit_event(RawEvent::Transfer(Some(owner), None, token_id));

        Ok(())
    }

    fn _transfer_from(from: T::AccountId, to: T::AccountId, token_id: T::Hash) -> Result {
        let owner = match Self::owner_of(token_id) {
            Some(c) => c,
            None => return Err("No owner for this token"),
        };

        ensure!(owner == from, "'from' account does not own this token");

        let balance_of_from = Self::balance_of(&from);
        let balance_of_to = Self::balance_of(&to);

        let new_balance_of_from = match balance_of_from.checked_sub(1) {
            Some (c) => c,
            None => return Err("Transfer causes underflow of 'from' token balance"),
        };

        let new_balance_of_to = match balance_of_to.checked_add(1) {
            Some(c) => c,
            None => return Err("Transfer causes overflow of 'to' token balance"),
        };
        
        Self::_clear_approval(token_id)?;
        <OwnedTokensCount<T>>::insert(&from, new_balance_of_from);
        <OwnedTokensCount<T>>::insert(&to, new_balance_of_to);
        <TokenOwner<T>>::insert(&token_id, &to);

        Self::deposit_event(RawEvent::Transfer(Some(from), Some(to), token_id));
        
        Ok(())
    }

    fn _clear_approval(token_id: T::Hash) -> Result{
        <TokenApprovals<T>>::remove(token_id);

        Ok(())
    }
}
