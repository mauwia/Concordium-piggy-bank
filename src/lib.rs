use concordium_std::*;

#[derive(Serialize, PartialEq, Eq, Debug, Clone, Copy)]
enum PiggyBankState {
    Intact,
    Smashed,
}
#[derive(Debug, PartialEq, Eq, Serial, Reject)]
enum SmashError {
    NotOwner,
    AlreadySmashed,
    TransferError, // Should never occur, see details below.
}
#[init(contract = "PiggyBanky")]
fn piggy_init<S: HasStateApi>(
    _ctx: &impl HasInitContext,
    _state_builder: &mut StateBuilder<S>,
) -> InitResult<PiggyBankState> {
    Ok(PiggyBankState::Intact)
}
#[receive(contract = "PiggyBanky", name = "insert", payable)]
fn piggy_insert<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<PiggyBankState, StateApiType = S>,
    amount: Amount,
) -> ReceiveResult<()> {
    ensure!(*host.state() == PiggyBankState::Intact);
    Ok(())
}
#[receive(contract = "PiggyBanky", name = "smash", mutable)]
fn piggy_smash<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<PiggyBankState, StateApiType = S>,
) -> Result<(), SmashError> {
    let owner = ctx.owner();
    let sender = ctx.sender();
    ensure!(sender.matches_account(&owner), SmashError::NotOwner);
    ensure!(
        *host.state() == PiggyBankState::Intact,
        SmashError::AlreadySmashed
    );
    *host.state_mut() = PiggyBankState::Smashed;
    let balance = host.self_balance();
    let transfer_result = host.invoke_transfer(&owner, balance);
    ensure!(transfer_result.is_ok(), SmashError::TransferError);
    Ok(())
}
#[receive(contract = "PiggyBanky", name = "view")]
fn view_piggy<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<PiggyBankState, StateApiType = S>,
) -> ReceiveResult<(PiggyBankState, Amount)> {
    let state = *host.state();
    let current_balance = host.self_balance();
    Ok((state, current_balance))
}
#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;
    #[concordium_test]
    fn test_init() {
        let ctx = TestInitContext::empty();
        let mut state_builder = TestStateBuilder::new();
        let state_result = piggy_init(&ctx, &mut state_builder);
        let state = state_result.expect_report("Contract initialization results in error.");
        println!("heelp");
        claim_eq!(
            state,
            PiggyBankState::Intact,
            "Piggy bank state should be intact after initialization."
        );
    }
    #[concordium_test]
    fn test_insert() {
        let ctx = TestReceiveContext::empty();
        let mut host = TestHost::new(PiggyBankState::Intact, TestStateBuilder::new());
        let amount = Amount::from_micro_ccd(100);
        let result = piggy_insert(&ctx, &host, amount);
        claim!(result.is_ok(), "Inserting CCD results in error");
    }
    #[concordium_test]
    fn test_smash() {
        let mut ctx = TestReceiveContext::empty();
        let owner = AccountAddress([0u8; 32]);
        ctx.set_owner(owner);
        let sender = Address::Account(owner);
        ctx.set_sender(sender);
        let mut host = TestHost::new(PiggyBankState::Intact, TestStateBuilder::new());
        let amount = Amount::from_micro_ccd(100);
        host.set_self_balance(amount);
        let result = piggy_smash(&ctx, &mut host);

        claim!(
            result.is_ok(),
            "Smashing intact piggy bank results in error."
        );
        claim_eq!(
            *host.state(),
            PiggyBankState::Smashed,
            "Piggy bank should be smashed."
        );
        claim_eq!(
            host.get_transfers(),
            [(owner, amount)],
            "Smashing did not produce the correct transfers."
        );
    }
    #[concordium_test]
    fn test_smash_if_not_owner() {
        let mut ctx = TestReceiveContext::empty();
        let owner = AccountAddress([0u8; 32]);
        ctx.set_owner(owner);

        let sender = Address::Account(AccountAddress([1u8; 32]));
        ctx.set_sender(sender);
        let mut host = TestHost::new(PiggyBankState::Intact, TestStateBuilder::new());
        let amount = Amount::from_micro_ccd(100);
        host.set_self_balance(amount);
        let result = piggy_smash(&ctx, &mut host);
        claim_eq!(result, Err(SmashError::NotOwner));
    }
}
