#![cfg(not(target_arch = "wasm32"))]

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_claim_ownership_msg, MockAppContract};
use andromeda_auction::mock::{
    mock_andromeda_auction, mock_auction_instantiate_msg, mock_start_auction, MockAuction,
};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg, MockCW721};

use andromeda_finance::splitter::AddressPercent;
use andromeda_modules::rates::{PercentRate, Rate, RateInfo};
use andromeda_rates::mock::{mock_andromeda_rates, mock_rates_instantiate_msg};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg,
};
use andromeda_std::{ado_base::Module, amp::Recipient, common::Milliseconds};
use andromeda_testing::{
    mock::mock_app, mock_builder::MockAndromedaBuilder, mock_contract::MockContract,
};
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Decimal, Uint128};
use cw_multi_test::Executor;

#[test]
fn test_auction_app_modules() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![]),
            ("buyer_one", vec![coin(1000, "uandr")]),
            ("buyer_two", vec![coin(1000, "uandr")]),
            ("recipient_one", vec![]),
            ("recipient_two", vec![]),
        ])
        .with_contracts(vec![
            ("cw721", mock_andromeda_cw721()),
            ("auction", mock_andromeda_auction()),
            ("app-contract", mock_andromeda_app()),
            ("rates", mock_andromeda_rates()),
            ("splitter", mock_andromeda_splitter()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");
    let buyer_two = andr.get_wallet("buyer_two");
    let recipient_one = andr.get_wallet("recipient_one");
    let recipient_two = andr.get_wallet("recipient_two");

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        None,
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let rates_init_msg = mock_rates_instantiate_msg(
        vec![RateInfo {
            is_additive: false,
            description: None,
            rate: Rate::Percent(PercentRate {
                percent: Decimal::from_ratio(1u32, 2u32),
            }),
            recipients: vec![
                Recipient::from_string("./splitter").with_msg(mock_splitter_send_msg())
            ],
        }],
        andr.kernel.addr(),
        None,
    );
    let rates_component =
        AppComponent::new("rates", "rates", to_json_binary(&rates_init_msg).unwrap());

    let splitter_init_msg = mock_splitter_instantiate_msg(
        vec![
            AddressPercent::new(
                Recipient::from_string(format!("{recipient_one}")),
                Decimal::from_ratio(1u8, 2u8),
            ),
            AddressPercent::new(
                Recipient::from_string(format!("{recipient_two}")),
                Decimal::from_ratio(1u8, 2u8),
            ),
        ],
        andr.kernel.addr(),
        None,
        None,
    );
    let splitter_component = AppComponent::new(
        "splitter",
        "splitter",
        to_json_binary(&splitter_init_msg).unwrap(),
    );

    let auction_init_msg = mock_auction_instantiate_msg(
        Some(vec![Module::new("rates", "./rates", false)]),
        andr.kernel.addr().to_string(),
        None,
        None,
    );
    let auction_component = AppComponent::new(
        "auction".to_string(),
        "auction".to_string(),
        to_json_binary(&auction_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![
        cw721_component.clone(),
        auction_component.clone(),
        rates_component,
        splitter_component,
    ];
    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Auction App",
        app_components,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(app.addr().clone()),
            &mock_claim_ownership_msg(None),
            &[],
        )
        .unwrap();

    // Mint Tokens
    let cw721: MockCW721 = app.query_ado_by_component_name(&router, cw721_component.name);
    cw721
        .execute_quick_mint(&mut router, owner.clone(), 1, owner.to_string())
        .unwrap();

    // Send Token to Auction
    let auction: MockAuction = app.query_ado_by_component_name(&router, auction_component.name);
    let start_time = Milliseconds::from_nanos(router.block_info().time.nanos())
        .plus_milliseconds(Milliseconds(100));
    let receive_msg = mock_start_auction(
        Some(start_time),
        start_time.plus_milliseconds(Milliseconds(1000)),
        "uandr".to_string(),
        None,
        None,
        None,
    );
    cw721
        .execute_send_nft(
            &mut router,
            owner.clone(),
            auction.addr(),
            "0",
            &receive_msg,
        )
        .unwrap();

    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: start_time.into(),
        chain_id: router.block_info().chain_id,
    });

    // Query Auction State
    let auction_ids: Vec<Uint128> =
        auction.query_auction_ids(&mut router, "0".to_string(), cw721.addr().to_string());

    assert_eq!(auction_ids.len(), 1);

    let auction_id = auction_ids.first().unwrap();
    let auction_state = auction.query_auction_state(&mut router, *auction_id);

    assert_eq!(auction_state.coin_denom, "uandr".to_string());
    assert_eq!(auction_state.owner, owner.to_string());

    // Place Bid One
    auction.execute_place_bid(
        &mut router,
        buyer_one.clone(),
        "0".to_string(),
        cw721.addr().to_string(),
        &[coin(50, "uandr")],
    );

    // Check Bid Status One
    let bids = auction.query_bids(&mut router, *auction_id);
    assert_eq!(bids.len(), 1);

    let bid = bids.first().unwrap();
    assert_eq!(bid.bidder, buyer_one.to_string());
    assert_eq!(bid.amount, Uint128::from(50u128));

    auction.execute_place_bid(
        &mut router,
        buyer_two.clone(),
        "0".to_string(),
        cw721.addr().to_string(),
        &[coin(100, "uandr")],
    );

    // Check Bid Status One
    let bids = auction.query_bids(&mut router, *auction_id);
    assert_eq!(bids.len(), 2);

    let bid_two = bids.get(1).unwrap();
    assert_eq!(bid_two.bidder, buyer_two.to_string());
    assert_eq!(bid_two.amount, Uint128::from(100u128));

    // End Auction
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: start_time.plus_milliseconds(Milliseconds(1000)).into(),
        chain_id: router.block_info().chain_id,
    });
    auction
        .execute_claim_auction(
            &mut router,
            buyer_two.clone(),
            "0".to_string(),
            cw721.addr().to_string(),
        )
        .unwrap();

    // Check Final State
    let token_owner = cw721.query_owner_of(&router, "0");
    assert_eq!(token_owner, buyer_two);
    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(owner_balance.amount, Uint128::from(50u128));
    let recipient_one_balance = router.wrap().query_balance(recipient_one, "uandr").unwrap();
    assert_eq!(recipient_one_balance.amount, Uint128::from(25u128));
    let recipient_two_balance = router.wrap().query_balance(recipient_two, "uandr").unwrap();
    assert_eq!(recipient_two_balance.amount, Uint128::from(25u128));
}

#[test]
fn test_auction_app_recipient() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![]),
            ("buyer_one", vec![coin(1000, "uandr")]),
            ("buyer_two", vec![coin(1000, "uandr")]),
            ("recipient_one", vec![]),
            ("recipient_two", vec![]),
        ])
        .with_contracts(vec![
            ("cw721", mock_andromeda_cw721()),
            ("auction", mock_andromeda_auction()),
            ("app-contract", mock_andromeda_app()),
            ("splitter", mock_andromeda_splitter()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");
    let buyer_two = andr.get_wallet("buyer_two");
    let recipient_one = andr.get_wallet("recipient_one");
    let recipient_two = andr.get_wallet("recipient_two");

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        None,
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let splitter_init_msg = mock_splitter_instantiate_msg(
        vec![
            AddressPercent::new(
                Recipient::from_string(format!("{recipient_one}")),
                Decimal::from_ratio(1u8, 2u8),
            ),
            AddressPercent::new(
                Recipient::from_string(format!("{recipient_two}")),
                Decimal::from_ratio(1u8, 2u8),
            ),
        ],
        andr.kernel.addr(),
        None,
        None,
    );
    let splitter_component = AppComponent::new(
        "splitter",
        "splitter",
        to_json_binary(&splitter_init_msg).unwrap(),
    );

    let auction_init_msg =
        mock_auction_instantiate_msg(None, andr.kernel.addr().to_string(), None, None);
    let auction_component = AppComponent::new(
        "auction".to_string(),
        "auction".to_string(),
        to_json_binary(&auction_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![
        cw721_component.clone(),
        auction_component.clone(),
        splitter_component,
    ];
    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Auction App",
        app_components,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(app.addr().clone()),
            &mock_claim_ownership_msg(None),
            &[],
        )
        .unwrap();

    // Mint Tokens
    let cw721: MockCW721 = app.query_ado_by_component_name(&router, cw721_component.name);
    cw721
        .execute_quick_mint(&mut router, owner.clone(), 1, owner.to_string())
        .unwrap();

    // Send Token to Auction
    let auction: MockAuction = app.query_ado_by_component_name(&router, auction_component.name);
    let start_time = Milliseconds::from_nanos(router.block_info().time.nanos())
        .plus_milliseconds(Milliseconds(100));
    let receive_msg = mock_start_auction(
        Some(start_time),
        start_time.plus_milliseconds(Milliseconds(1000)),
        "uandr".to_string(),
        None,
        None,
        Some(Recipient::from_string("./splitter").with_msg(mock_splitter_send_msg())),
    );
    cw721
        .execute_send_nft(
            &mut router,
            owner.clone(),
            auction.addr(),
            "0",
            &receive_msg,
        )
        .unwrap();

    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: start_time.into(),
        chain_id: router.block_info().chain_id,
    });

    // Query Auction State
    let auction_ids: Vec<Uint128> =
        auction.query_auction_ids(&mut router, "0".to_string(), cw721.addr().to_string());

    assert_eq!(auction_ids.len(), 1);

    let auction_id = auction_ids.first().unwrap();
    let auction_state = auction.query_auction_state(&mut router, *auction_id);

    assert_eq!(auction_state.coin_denom, "uandr".to_string());
    assert_eq!(auction_state.owner, owner.to_string());

    // Place Bid One
    auction.execute_place_bid(
        &mut router,
        buyer_one.clone(),
        "0".to_string(),
        cw721.addr().to_string(),
        &[coin(50, "uandr")],
    );

    // Check Bid Status One
    let bids = auction.query_bids(&mut router, *auction_id);
    assert_eq!(bids.len(), 1);

    let bid = bids.first().unwrap();
    assert_eq!(bid.bidder, buyer_one.to_string());
    assert_eq!(bid.amount, Uint128::from(50u128));

    auction.execute_place_bid(
        &mut router,
        buyer_two.clone(),
        "0".to_string(),
        cw721.addr().to_string(),
        &[coin(100, "uandr")],
    );

    // Check Bid Status One
    let bids = auction.query_bids(&mut router, *auction_id);
    assert_eq!(bids.len(), 2);

    let bid_two = bids.get(1).unwrap();
    assert_eq!(bid_two.bidder, buyer_two.to_string());
    assert_eq!(bid_two.amount, Uint128::from(100u128));

    // End Auction
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: start_time.plus_milliseconds(Milliseconds(1000)).into(),
        chain_id: router.block_info().chain_id,
    });
    auction
        .execute_claim_auction(
            &mut router,
            buyer_two.clone(),
            "0".to_string(),
            cw721.addr().to_string(),
        )
        .unwrap();

    // Check Final State
    let token_owner = cw721.query_owner_of(&router, "0");
    assert_eq!(token_owner, buyer_two);
    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(owner_balance.amount, Uint128::zero());
    let recipient_one_balance = router.wrap().query_balance(recipient_one, "uandr").unwrap();
    assert_eq!(recipient_one_balance.amount, Uint128::from(50u128));
    let recipient_two_balance = router.wrap().query_balance(recipient_two, "uandr").unwrap();
    assert_eq!(recipient_two_balance.amount, Uint128::from(50u128));
}
