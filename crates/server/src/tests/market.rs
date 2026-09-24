//! M1.5 / M2.5 — the marketplace foundation and the negotiation on top of it.
//!
//! Everything here is a unit test over the pure halves of M1 and M2: the `[market]` validator, the
//! `?scope=` union rule, the slug/label rules the admin editor enforces, the audit payload, every
//! market filter on `/api/posts`, and — for M2 — the offer body contract, the currency precedence
//! rule, the deal-transition matrix and the append-only guarantee. That split is deliberate: a
//! test that needs a live Postgres does not run under `cargo test --workspace`, so the behaviour
//! that needs a database (the actual 403, the actual `audit_events` row, the actual `posts.sold_at`
//! after a completed deal) belongs to the card's runtime curl gate, and what belongs here is the
//! logic those responses are computed from.

/// M1.1 — `[market] default_currency`.
#[cfg(test)]
mod market_config_tests {
    use crate::config::{is_currency_code, Config};

    /// SPEC B7: unset by default. There is no currency that is right for a server that has not
    /// chosen one, and inventing a plausible-looking `USD` would price every listing on a node in
    /// a unit nobody picked — while looking, from the outside, exactly like a deliberate choice.
    #[test]
    fn default_currency_is_unset_by_default() {
        let config: Config = toml::from_str("").expect("parse empty config");
        assert!(
            config.market.default_currency.is_none(),
            "a server must not be handed a currency it never configured"
        );
        config.validate_market().expect("an unset default is valid");
    }

    #[test]
    fn a_well_formed_default_currency_is_accepted() {
        for code in ["USD", "EUR", "GBP", "XOF"] {
            let config: Config =
                toml::from_str(&format!("[market]\ndefault_currency = \"{code}\"\n"))
                    .expect("parse config");
            assert_eq!(config.market.default_currency.as_deref(), Some(code));
            config
                .validate_market()
                .unwrap_or_else(|e| panic!("{code} must be accepted: {e}"));
        }
    }

    /// The card's four rejects plus two more of the same shape. The message has to name the key
    /// and echo the value, because the operator's next move is to find that line in a TOML file.
    #[test]
    fn a_malformed_default_currency_refuses_to_start_and_names_the_key_and_the_value() {
        for bad in ["usd", "US", "USDD", "", "U5D", "us d"] {
            let config: Config =
                toml::from_str(&format!("[market]\ndefault_currency = \"{bad}\"\n"))
                    .expect("parse config");

            let err = config
                .validate_market()
                .expect_err("a malformed currency must refuse to start")
                .to_string();

            assert!(
                err.contains("[market] default_currency"),
                "the error must name the key, got: {err}"
            );
            assert!(
                err.contains(&format!("{bad:?}")),
                "the error must quote the bad value {bad:?}, got: {err}"
            );
        }
    }

    /// SPEC B7, the precedence that the whole key exists to express.
    #[test]
    fn a_listings_own_currency_always_wins_over_the_server_default() {
        let configured: Config =
            toml::from_str("[market]\ndefault_currency = \"EUR\"\n").expect("parse config");
        assert_eq!(
            configured.market.resolve_currency(Some("USD")).as_deref(),
            Some("USD"),
            "the listing's own currency must not be overwritten by the server's"
        );
        assert_eq!(
            configured.market.resolve_currency(None).as_deref(),
            Some("EUR"),
            "the default is the fallback for a post that arrives without one"
        );

        let unset: Config = toml::from_str("").expect("parse empty config");
        assert_eq!(unset.market.resolve_currency(Some("USD")).as_deref(), Some("USD"));
        assert_eq!(
            unset.market.resolve_currency(None),
            None,
            "with no default configured, a listing without a currency keeps none"
        );
    }

    /// The Rust rule and `chk_posts_currency` (`currency ~ '^[A-Z]{3}$'`) have to agree, or the
    /// server accepts values the database then rejects as a 500 on somebody's listing.
    #[test]
    fn the_currency_rule_is_the_one_the_database_enforces() {
        for good in ["USD", "EUR", "XOF", "AAA"] {
            assert!(is_currency_code(good), "{good:?} must be accepted");
        }
        // Lowercase, too short, too long, empty, a digit, punctuation, and a multi-byte
        // character that happens to be three chars but not three bytes.
        for bad in ["usd", "US", "USDD", "", "U5D", "US$", "\u{20ac}UR"] {
            assert!(!is_currency_code(bad), "{bad:?} must be rejected");
        }
    }
}

/// M1.2 / M1.3 — the categories API: scope semantics, the slug contract, the audit payload.
#[cfg(test)]
mod categories_tests {
    use uuid::Uuid;

    use komun_core::models::{Category, CategoryScope};

    use crate::api::categories::{
        accepted_scopes, audit_detail_create, audit_detail_update, bad_request, parse_scope,
        validate_label, validate_slug, AUDIT_CREATE, AUDIT_UPDATE,
    };
    use crate::auth::AuthUser;

    const SCHEMA: &str = include_str!("../../../../migrations/001_schema.sql");

    /// `(slug, scope)` for every seeded category, read out of the migration rather than copied
    /// into this file. Re-scoping a category is a one-line `UPDATE` to the seed (SPEC 1.6), and
    /// this test should follow it rather than have to be edited alongside it.
    fn seeded_scopes() -> Vec<(String, CategoryScope)> {
        let start = SCHEMA
            .find("INSERT INTO categories")
            .expect("the migration seeds the categories table");
        let block = &SCHEMA[start..];
        let block = &block[..block.find(';').expect("the seed INSERT is terminated")];

        block
            .lines()
            .filter(|line| line.trim_start().starts_with('('))
            .map(|line| {
                // Each row is `('slug', 'Label', 'scope', N),` and no seeded label contains an
                // apostrophe, so the odd-indexed pieces of a split on `'` are the three values.
                let fields: Vec<&str> = line.split('\'').skip(1).step_by(2).collect();
                assert_eq!(fields.len(), 3, "unexpected seed row: {line}");
                let scope = CategoryScope::parse(fields[2])
                    .unwrap_or_else(|| panic!("seed row has an unknown scope: {line}"));
                (fields[0].to_string(), scope)
            })
            .collect()
    }

    /// The union rule `?scope=` implements, mirroring the SQL predicate in
    /// `db::categories::list`: `scope = $1 OR scope = 'both'`. Keep the two identical.
    fn scope_includes(requested: CategoryScope, row: CategoryScope) -> bool {
        row == requested || row == CategoryScope::Both
    }

    fn slugs_for(requested: CategoryScope) -> Vec<String> {
        seeded_scopes()
            .into_iter()
            .filter(|(_, scope)| scope_includes(requested, *scope))
            .map(|(slug, _)| slug)
            .collect()
    }

    fn category(
        slug: &str,
        label: &str,
        scope: CategoryScope,
        sort_order: i32,
        active: bool,
    ) -> Category {
        let now = chrono::Utc::now();
        Category {
            slug: slug.to_string(),
            label: label.to_string(),
            scope,
            sort_order,
            active,
            created_at: now,
            updated_at: now,
        }
    }

    fn actor(role: &str) -> AuthUser {
        AuthUser {
            user_id: Uuid::now_v7(),
            session_id: Uuid::now_v7(),
            role: role.to_string(),
            email_verified: true,
        }
    }

    /// The whole point of the `scope` column: `market` is not an equality test. A `both` category
    /// like `services` has to reach the market form, or half the taxonomy is unreachable from it.
    #[test]
    fn scope_market_is_a_union_with_both_not_an_equality() {
        assert_eq!(seeded_scopes().len(), 23, "SPEC 1.6 seeds 23 categories");

        let market = slugs_for(CategoryScope::Market);
        assert_eq!(
            market.len(),
            21,
            "the market form must see all 21 market-usable categories, got {market:?}"
        );
        assert!(
            market.iter().any(|s| s == "electronics"),
            "a market-only category is missing"
        );
        assert!(
            market.iter().any(|s| s == "services"),
            "a 'both' category must be offered to the market form"
        );
        assert!(
            !market.iter().any(|s| s == "shelter"),
            "an aid-only category must not reach the market form"
        );
    }

    #[test]
    fn scope_aid_sees_its_own_rows_and_the_shared_ones() {
        let aid = slugs_for(CategoryScope::Aid);
        assert_eq!(aid.len(), 8, "the aid form must see 8 categories, got {aid:?}");
        assert!(aid.iter().any(|s| s == "shelter"), "an aid-only category is missing");
        assert!(aid.iter().any(|s| s == "food"), "a 'both' category must reach the aid form");
        assert!(
            !aid.iter().any(|s| s == "electronics"),
            "a market-only category must not reach the aid form"
        );
    }

    /// `scope=both` narrows to the shared rows, which falls straight out of the same rule.
    #[test]
    fn scope_both_narrows_to_the_shared_rows() {
        let both = slugs_for(CategoryScope::Both);
        assert_eq!(both.len(), 6, "6 categories are shared, got {both:?}");
    }

    #[test]
    fn a_missing_or_blank_scope_means_every_active_category() {
        assert_eq!(parse_scope(None), Ok(None));
        assert_eq!(parse_scope(Some("   ")), Ok(None), "an untouched form field is not a filter");
    }

    #[test]
    fn every_scope_the_check_allows_parses() {
        for scope in CategoryScope::ALL {
            assert_eq!(parse_scope(Some(scope.as_str())), Ok(Some(*scope)));
        }
    }

    /// `?scope=nope` answering `200 []` would be indistinguishable from a server with no
    /// categories, and would leave the caller with no way to find the typo.
    #[test]
    fn an_unknown_scope_is_rejected_and_the_message_lists_what_is_accepted() {
        let err = parse_scope(Some("nope")).expect_err("an unknown scope must not be accepted");
        for scope in CategoryScope::ALL {
            assert!(err.contains(scope.as_str()), "the 400 must list {scope}, got: {err}");
        }
        assert!(err.contains("nope"), "the 400 must echo the bad value, got: {err}");
        assert_eq!(accepted_scopes(), "aid, market, both");
    }

    /// The message assertions above cannot see a status code; this pins the mapping every one of
    /// those messages goes through on its way out of a handler.
    #[test]
    fn a_rejected_parameter_leaves_as_a_400() {
        use axum::response::IntoResponse;

        let response = bad_request("scope must be one of aid, market, both").into_response();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    /// M1.3 is admin **or** superadmin, which is exactly why the category routes carry their own
    /// `require_admin` layer instead of being mounted on `api::admin`'s superadmin-only router.
    /// This pins the predicate that middleware branches on; the 403 itself is the curl gate's.
    #[test]
    fn only_admins_and_superadmins_may_edit_the_taxonomy() {
        for (role, allowed) in [
            ("user", false),
            ("admin", true),
            ("superadmin", true),
            ("", false),
            ("Admin", false),
            ("administrator", false),
        ] {
            assert_eq!(
                actor(role).is_admin(),
                allowed,
                "role {role:?} must {} edit categories",
                if allowed { "be able to" } else { "not be able to" }
            );
        }
    }

    /// `audit_events.subject_id` is a `UUID` column and a category is keyed by a TEXT slug, so
    /// the subject column is always NULL for these two actions and the slug has to travel in
    /// `detail`. A row that did not carry it would record that *something* changed and not what.
    #[test]
    fn a_category_audit_row_carries_the_slug_the_subject_column_cannot_hold() {
        let created = category("bicycle-parts", "Bicycle Parts", CategoryScope::Market, 15, true);
        let detail = audit_detail_create(&created);

        assert_eq!(detail["slug"], "bicycle-parts");
        assert_eq!(detail["label"], "Bicycle Parts");
        assert_eq!(detail["scope"], "market");
        assert_eq!(detail["sort_order"], 15);
        assert_eq!(detail["active"], true);
    }

    #[test]
    fn an_update_audit_row_records_both_sides_and_the_reindex() {
        let before = category("household", "Household Goods", CategoryScope::Market, 140, true);
        let after = category("household", "Home Goods", CategoryScope::Market, 140, false);
        let detail = audit_detail_update("household", &before, &after, 7);

        assert_eq!(detail["slug"], "household");
        assert_eq!(detail["from"]["label"], "Household Goods");
        assert_eq!(detail["to"]["label"], "Home Goods");
        assert_eq!(detail["from"]["active"], true);
        assert_eq!(detail["to"]["active"], false);
        // SPEC 1.6: the FTS trigger indexes the label, so a rename has to re-run it for the
        // category's posts. Recording the count is what makes a rename that skipped the reindex
        // visible afterwards instead of showing up as posts nobody can find.
        assert_eq!(detail["posts_reindexed"], 7);
    }

    #[test]
    fn the_audit_actions_are_namespaced_like_the_other_admin_actions() {
        assert_eq!(AUDIT_CREATE, "admin.category_create");
        assert_eq!(AUDIT_UPDATE, "admin.category_update");
        assert!(AUDIT_CREATE.starts_with("admin."), "one prefix must find every admin action");
        assert!(AUDIT_UPDATE.starts_with("admin."));
    }

    #[test]
    fn slugs_are_lowercase_kebab() {
        for good in ["electronics", "bikes-vehicles", "baby-kids", "web3", "a"] {
            validate_slug(good).unwrap_or_else(|e| panic!("{good:?} must be accepted: {e}"));
        }
        for bad in [
            "",
            "Electronics",
            "bikes vehicles",
            "bikes_vehicles",
            "-leading",
            "trailing-",
            "double--dash",
            "caf\u{e9}",
            "a/b",
        ] {
            assert!(validate_slug(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    /// The rule the admin editor applies to a new slug has to be one every existing slug already
    /// satisfies, or the seeded taxonomy is something the editor could not have produced.
    #[test]
    fn every_seeded_slug_passes_the_rule_the_admin_editor_enforces() {
        for (slug, _) in seeded_scopes() {
            validate_slug(&slug)
                .unwrap_or_else(|e| panic!("seeded slug {slug:?} is rejected: {e}"));
        }
    }

    #[test]
    fn labels_must_not_be_blank() {
        validate_label("Electronics & Computers").expect("a seeded label must be accepted");
        assert!(validate_label("").is_err());
        assert!(validate_label("   ").is_err(), "whitespace is not a label");
        validate_label(&"x".repeat(80)).expect("80 characters is the limit, not past it");
        assert!(validate_label(&"x".repeat(81)).is_err());
    }
}

/// M1.4 — the market filters on `GET /api/posts`.
#[cfg(test)]
mod post_filter_tests {
    use komun_core::models::{ItemCondition, PostKind, PostStatus};

    use crate::api::posts::{validate_filters, PostFilters};
    use crate::db::posts::{DEFAULT_LIMIT, MAX_LIMIT};

    #[test]
    fn an_empty_query_string_is_the_default_page_of_everything() {
        let filter = validate_filters(&PostFilters::default()).expect("no filters is legal");
        assert_eq!(filter.kind, None);
        assert_eq!(filter.category, None);
        assert_eq!(filter.currency, None);
        assert_eq!(filter.limit, DEFAULT_LIMIT, "an unbounded feed is not an option");
        assert_eq!(filter.offset, 0);
    }

    /// Every one of these is what an untouched `<select>` or `<input>` submits.
    #[test]
    fn a_blank_parameter_is_no_filter_rather_than_a_rejection() {
        let raw = PostFilters {
            kind: Some(String::new()),
            currency: Some("  ".to_string()),
            min_price_cents: Some(String::new()),
            item_condition: Some(String::new()),
            limit: Some(String::new()),
            ..Default::default()
        };

        let filter = validate_filters(&raw).expect("blank parameters must not be errors");
        assert_eq!(filter.kind, None);
        assert_eq!(filter.currency, None);
        assert_eq!(filter.min_price_cents, None);
        assert_eq!(filter.item_condition, None);
        assert_eq!(filter.limit, DEFAULT_LIMIT);
    }

    #[test]
    fn the_kind_filter_accepts_the_market_kinds_and_names_itself_when_it_cannot() {
        for kind in [PostKind::Listing, PostKind::Want] {
            let raw = PostFilters {
                kind: Some(kind.as_str().to_string()),
                ..Default::default()
            };
            assert_eq!(
                validate_filters(&raw).expect("a market kind is legal").kind,
                Some(kind)
            );
        }

        let raw = PostFilters {
            kind: Some("listings".to_string()),
            ..Default::default()
        };
        let err = validate_filters(&raw).expect_err("an unknown kind must be a 400, not an empty list");
        assert!(err.starts_with("kind "), "the 400 must name the parameter, got: {err}");
        assert!(err.contains("listing"), "the 400 must list the accepted values, got: {err}");
        assert!(err.contains("listings"), "the 400 must echo the bad value, got: {err}");
    }

    /// Shape only. A well-formed slug nobody has created yet is a legitimately empty result;
    /// `Bikes & Vehicles` is a label pasted into a slug field and can never match anything.
    #[test]
    fn the_category_filter_takes_a_slug_and_rejects_what_could_not_be_one() {
        let raw = PostFilters {
            category: Some("bikes-vehicles".to_string()),
            ..Default::default()
        };
        assert_eq!(
            validate_filters(&raw).expect("a slug is legal").category.as_deref(),
            Some("bikes-vehicles")
        );

        let raw = PostFilters {
            category: Some("Bikes & Vehicles".to_string()),
            ..Default::default()
        };
        let err = validate_filters(&raw).expect_err("a label is not a slug");
        assert!(err.starts_with("category "), "the 400 must name the parameter, got: {err}");
    }

    #[test]
    fn the_price_filters_take_whole_cents_and_reject_anything_else() {
        let raw = PostFilters {
            min_price_cents: Some("100".to_string()),
            max_price_cents: Some("25000".to_string()),
            ..Default::default()
        };
        let filter = validate_filters(&raw).expect("a cent range is legal");
        assert_eq!(filter.min_price_cents, Some(100));
        assert_eq!(filter.max_price_cents, Some(25_000));

        // A decimal, a word, and a negative bound: none of them is a number of cents.
        let cases = [
            ("min_price_cents", "12.50"),
            ("min_price_cents", "cheap"),
            ("min_price_cents", "-1"),
            ("max_price_cents", "1e5"),
            ("max_price_cents", "\u{a3}20"),
        ];
        for (field, value) in cases {
            let raw = if field == "min_price_cents" {
                PostFilters {
                    min_price_cents: Some(value.to_string()),
                    ..Default::default()
                }
            } else {
                PostFilters {
                    max_price_cents: Some(value.to_string()),
                    ..Default::default()
                }
            };
            let err = validate_filters(&raw).expect_err("a malformed price must be a 400");
            assert!(err.starts_with(field), "the 400 must name {field}, got: {err}");
        }
    }

    /// An inverted range can only ever match nothing, so `200 []` would be a true and useless
    /// answer to what is plainly a mistake.
    #[test]
    fn an_inverted_price_range_is_rejected_rather_than_answered_with_nothing() {
        let raw = PostFilters {
            min_price_cents: Some("5000".to_string()),
            max_price_cents: Some("100".to_string()),
            ..Default::default()
        };
        let err = validate_filters(&raw).expect_err("min above max must be a 400");
        assert!(
            err.contains("min_price_cents") && err.contains("max_price_cents"),
            "the 400 must name both parameters, got: {err}"
        );
    }

    #[test]
    fn the_currency_filter_is_iso_4217_or_a_400() {
        let raw = PostFilters {
            currency: Some("USD".to_string()),
            ..Default::default()
        };
        assert_eq!(
            validate_filters(&raw).expect("USD is legal").currency.as_deref(),
            Some("USD")
        );

        for bad in ["dollars", "usd", "US", "USDD"] {
            let raw = PostFilters {
                currency: Some(bad.to_string()),
                ..Default::default()
            };
            let err = validate_filters(&raw).expect_err("a non-ISO currency must be a 400");
            assert!(err.starts_with("currency "), "the 400 must name the parameter, got: {err}");
            assert!(err.contains(bad), "the 400 must echo the bad value, got: {err}");
        }
    }

    #[test]
    fn the_item_condition_filter_matches_the_schemas_check_list() {
        for condition in ItemCondition::ALL {
            let raw = PostFilters {
                item_condition: Some(condition.as_str().to_string()),
                ..Default::default()
            };
            assert_eq!(
                validate_filters(&raw)
                    .expect("a known condition is legal")
                    .item_condition,
                Some(*condition)
            );
        }

        let raw = PostFilters {
            item_condition: Some("mint".to_string()),
            ..Default::default()
        };
        let err = validate_filters(&raw).expect_err("'mint' is not a condition this schema allows");
        assert!(err.starts_with("item_condition "), "the 400 must name the parameter, got: {err}");
        assert!(err.contains("like_new"), "the 400 must list the accepted values, got: {err}");
    }

    /// `status` predates M1, but it is the same trap: a value the `CHECK` has never heard of used
    /// to filter to nothing and answer `200 []`.
    #[test]
    fn the_status_filter_is_validated_too_so_a_typo_is_not_an_empty_marketplace() {
        let raw = PostFilters {
            status: Some("active".to_string()),
            ..Default::default()
        };
        assert_eq!(
            validate_filters(&raw).expect("active is legal").status,
            Some(PostStatus::Active)
        );

        let raw = PostFilters {
            status: Some("sold".to_string()),
            ..Default::default()
        };
        let err = validate_filters(&raw).expect_err("'sold' is not a post status");
        assert!(err.starts_with("status "), "the 400 must name the parameter, got: {err}");
    }

    /// Rejected rather than clamped: a caller that asks for 5,000 and is quietly handed 200 has
    /// no way to know its pagination is wrong.
    #[test]
    fn limit_and_offset_are_bounded_and_named_when_they_are_not() {
        let raw = PostFilters {
            limit: Some("20".to_string()),
            offset: Some("40".to_string()),
            ..Default::default()
        };
        let filter = validate_filters(&raw).expect("a page is legal");
        assert_eq!(filter.limit, 20);
        assert_eq!(filter.offset, 40);

        let raw = PostFilters {
            limit: Some(MAX_LIMIT.to_string()),
            ..Default::default()
        };
        assert_eq!(
            validate_filters(&raw).expect("the maximum is legal").limit,
            MAX_LIMIT
        );

        for bad in [
            "0".to_string(),
            "-5".to_string(),
            (MAX_LIMIT + 1).to_string(),
            "all".to_string(),
        ] {
            let raw = PostFilters {
                limit: Some(bad.clone()),
                ..Default::default()
            };
            let err = validate_filters(&raw).expect_err("a bad limit must be a 400");
            assert!(err.starts_with("limit "), "the 400 must name the parameter for {bad:?}, got: {err}");
        }

        let raw = PostFilters {
            offset: Some("-1".to_string()),
            ..Default::default()
        };
        let err = validate_filters(&raw).expect_err("a negative offset must be a 400");
        assert!(err.starts_with("offset "), "the 400 must name the parameter, got: {err}");
    }

    /// The query the market grid actually sends, all filters at once.
    #[test]
    fn a_market_view_can_ask_for_everything_it_needs_in_one_request() {
        let raw = PostFilters {
            kind: Some("listing".to_string()),
            category: Some("electronics".to_string()),
            min_price_cents: Some("100".to_string()),
            max_price_cents: Some("50000".to_string()),
            currency: Some("USD".to_string()),
            item_condition: Some("like_new".to_string()),
            q: Some("laptop".to_string()),
            limit: Some("24".to_string()),
            offset: Some("24".to_string()),
            ..Default::default()
        };

        let filter = validate_filters(&raw).expect("the market view's own query must be legal");
        assert_eq!(filter.kind, Some(PostKind::Listing));
        assert_eq!(filter.category.as_deref(), Some("electronics"));
        assert_eq!(filter.min_price_cents, Some(100));
        assert_eq!(filter.max_price_cents, Some(50_000));
        assert_eq!(filter.currency.as_deref(), Some("USD"));
        assert_eq!(filter.item_condition, Some(ItemCondition::LikeNew));
        assert_eq!(filter.q.as_deref(), Some("laptop"));
        assert_eq!(filter.limit, 24);
        assert_eq!(filter.offset, 24);
    }
}

/// M2.5 — the negotiation and the deal lifecycle.
#[cfg(test)]
mod offer_tests {
    use uuid::Uuid;

    use komun_core::models::{MatchStatus, OfferKind, PostKind};

    use crate::api::conversations::{
        offer_kinds, offers_allowed_on, parse_status, resolve_offer_currency, validate_offer,
        OfferRequest, MAX_NOTE_CHARS,
    };
    use crate::db::conversations::{
        check_accept_actor, check_accept_allowed, check_transition, Thread, NOTHING_TO_ACCEPT,
        SELF_ACCEPT,
    };

    fn request(kind: &str) -> OfferRequest {
        OfferRequest {
            kind: Some(kind.to_string()),
            ..Default::default()
        }
    }

    fn priced(kind: &str, amount_cents: i64) -> OfferRequest {
        OfferRequest {
            kind: Some(kind.to_string()),
            amount_cents: Some(amount_cents),
            ..Default::default()
        }
    }

    // -----------------------------------------------------------------------
    // M2.1 — who may post an offer, and on what
    // -----------------------------------------------------------------------

    /// The M2 decision the aid half of the product depends on: an aid conversation keeps its plain
    /// propose/accept flow, and nothing about it invites a price.
    #[test]
    fn offers_are_for_listings_and_wanted_ads_and_an_aid_thread_says_so() {
        for kind in [PostKind::Listing, PostKind::Want] {
            offers_allowed_on(kind)
                .unwrap_or_else(|e| panic!("a {kind} thread must accept offers: {e}"));
        }

        for kind in [PostKind::Resource, PostKind::Need, PostKind::Offer] {
            let err = offers_allowed_on(kind)
                .expect_err("an aid thread must refuse offers");
            assert_eq!(err, "offers are for listings and wanted ads");
        }
    }

    /// `PostKind::is_market` is the single source of that split, so a sixth kind added later lands
    /// on one side of it by construction rather than by someone remembering this rule exists.
    #[test]
    fn every_kind_the_schema_allows_is_on_one_side_of_that_line() {
        for kind in PostKind::ALL {
            assert_eq!(
                offers_allowed_on(*kind).is_ok(),
                kind.is_market(),
                "{kind} must be allowed exactly when it is a marketplace kind"
            );
        }
    }

    /// A non-participant is refused by the thread, not by the offer body — which is why the check
    /// is one predicate over the two ids on the row and not a query per route.
    #[test]
    fn only_the_two_people_on_a_thread_are_participants() {
        let author = Uuid::now_v7();
        let responder = Uuid::now_v7();
        let stranger = Uuid::now_v7();

        let thread = Thread {
            responder_id: responder,
            author_id: author,
            post_kind: PostKind::Listing,
            post_currency: Some("USD".to_string()),
        };

        assert!(thread.is_participant(author), "the post's author is a participant");
        assert!(thread.is_participant(responder), "the responder is a participant");
        assert!(
            !thread.is_participant(stranger),
            "anyone else must be refused: this is the 403"
        );
    }

    // -----------------------------------------------------------------------
    // M2.1 — the offer body
    // -----------------------------------------------------------------------

    #[test]
    fn every_kind_the_check_allows_is_accepted_and_nothing_else_is() {
        for kind in [OfferKind::Offer, OfferKind::Counter] {
            let valid = validate_offer(&priced(kind.as_str(), 2500))
                .unwrap_or_else(|e| panic!("{kind} must be accepted: {e}"));
            assert_eq!(valid.kind, kind);
        }
        assert_eq!(
            validate_offer(&priced("accept", 2000)).expect("accept is a kind").kind,
            OfferKind::Accept
        );
        assert_eq!(
            validate_offer(&request("decline")).expect("decline is a kind").kind,
            OfferKind::Decline
        );

        // `bid` is the value SPEC A1's negative-insert list proves the database rejects; the API
        // has to reject it first, with a message that says what the four steps are.
        let err = validate_offer(&priced("bid", 100)).expect_err("'bid' is not an offer kind");
        for kind in OfferKind::ALL {
            assert!(err.contains(kind.as_str()), "the 400 must list {kind}, got: {err}");
        }
        assert!(err.contains("bid"), "the 400 must echo the bad value, got: {err}");
        assert_eq!(offer_kinds(), "offer, counter, accept, decline");
    }

    #[test]
    fn a_missing_kind_is_a_400_that_says_what_the_field_takes() {
        for raw in [OfferRequest::default(), request(""), request("   ")] {
            let err = validate_offer(&raw).expect_err("an offer with no kind is not an offer");
            assert!(err.contains("kind"), "the 400 must name the field, got: {err}");
            assert!(err.contains("counter"), "the 400 must list the kinds, got: {err}");
        }
    }

    /// `offer`, `counter` and `accept` are all statements about a number; without one there is
    /// nothing on the table and nothing for the other side to agree to.
    #[test]
    fn the_three_kinds_that_carry_a_number_require_one() {
        for kind in [OfferKind::Offer, OfferKind::Counter, OfferKind::Accept] {
            let err = validate_offer(&request(kind.as_str()))
                .expect_err("a priced step with no amount must be a 400");
            assert!(
                err.contains("amount_cents"),
                "the 400 must name the field, got: {err}"
            );
            assert!(err.contains(kind.as_str()), "the 400 must name the kind, got: {err}");
        }
    }

    /// A decline refuses the number already on the thread. A second copy of one under `decline`
    /// would read as a counter to anybody rendering the trail.
    #[test]
    fn a_decline_carries_no_amount() {
        let err = validate_offer(&priced("decline", 2000))
            .expect_err("a decline with an amount must be a 400");
        assert!(err.contains("amount_cents"), "the 400 must name the field, got: {err}");
        assert!(err.contains("decline"), "the 400 must name the kind, got: {err}");

        let valid = validate_offer(&request("decline")).expect("a bare decline is legal");
        assert_eq!(valid.amount_cents, None);
    }

    /// Zero is a real offer — "take it, it's free" — and `chk_match_offers_amount` allows it.
    /// Negative is not, in either place.
    #[test]
    fn an_amount_may_be_zero_but_never_negative() {
        assert_eq!(
            validate_offer(&priced("offer", 0)).expect("free is a price").amount_cents,
            Some(0)
        );

        for cents in [-1, -2500, i64::MIN] {
            let err = validate_offer(&priced("offer", cents))
                .expect_err("a negative amount must be a 400");
            assert!(
                err.contains("amount_cents") && err.contains("negative"),
                "the 400 must name the field and the reason, got: {err}"
            );
        }
    }

    /// The same rule `chk_match_offers_currency` enforces, applied first so a bad code is a 400
    /// rather than a constraint violation surfacing as a 500.
    #[test]
    fn an_offers_own_currency_is_iso_4217_or_a_400() {
        let valid = validate_offer(&OfferRequest {
            currency: Some("EUR".to_string()),
            ..priced("offer", 2500)
        })
        .expect("EUR is legal");
        assert_eq!(valid.currency.as_deref(), Some("EUR"));

        for bad in ["usd", "US", "USDD", "dollars", "\u{20ac}"] {
            let err = validate_offer(&OfferRequest {
                currency: Some(bad.to_string()),
                ..priced("offer", 2500)
            })
            .expect_err("a non-ISO currency must be a 400");
            assert!(err.contains("currency"), "the 400 must name the field, got: {err}");
            assert!(err.contains(bad), "the 400 must echo the bad value, got: {err}");
        }

        // An untouched form field is not a currency the caller chose; it falls through to the
        // precedence rule rather than being rejected.
        let valid = validate_offer(&OfferRequest {
            currency: Some("  ".to_string()),
            ..priced("offer", 2500)
        })
        .expect("a blank currency is no currency");
        assert_eq!(valid.currency, None);
    }

    #[test]
    fn a_note_is_trimmed_bounded_and_optional() {
        let valid = validate_offer(&OfferRequest {
            note: Some("  collection only, weekends  ".to_string()),
            ..priced("offer", 2500)
        })
        .expect("a note is legal");
        assert_eq!(valid.note.as_deref(), Some("collection only, weekends"));

        for blank in ["", "   ", "\n\t"] {
            let valid = validate_offer(&OfferRequest {
                note: Some(blank.to_string()),
                ..priced("offer", 2500)
            })
            .expect("a blank note is no note");
            assert_eq!(valid.note, None, "a whitespace-only note must not be stored");
        }

        let at_limit = "x".repeat(MAX_NOTE_CHARS);
        assert_eq!(
            validate_offer(&OfferRequest {
                note: Some(at_limit.clone()),
                ..priced("offer", 2500)
            })
            .expect("the limit itself is legal")
            .note,
            Some(at_limit)
        );

        let err = validate_offer(&OfferRequest {
            note: Some("x".repeat(MAX_NOTE_CHARS + 1)),
            ..priced("offer", 2500)
        })
        .expect_err("one character past the limit must be a 400");
        assert!(err.contains("note"), "the 400 must name the field, got: {err}");
        assert!(
            err.contains(&MAX_NOTE_CHARS.to_string()),
            "the 400 must state the limit, got: {err}"
        );
    }

    /// Characters, not bytes. 500 Cyrillic characters are 1,000 bytes; a byte limit would cut a
    /// note in half for half the world while the message still promised 500.
    #[test]
    fn the_note_limit_counts_characters_not_bytes() {
        let cyrillic = "\u{434}".repeat(MAX_NOTE_CHARS);
        assert!(
            cyrillic.len() > MAX_NOTE_CHARS,
            "the fixture must actually be longer in bytes than in characters"
        );
        validate_offer(&OfferRequest {
            note: Some(cyrillic),
            ..priced("offer", 2500)
        })
        .expect("500 characters is 500 characters in any script");
    }

    // -----------------------------------------------------------------------
    // M2 — currency precedence
    // -----------------------------------------------------------------------

    /// The offer's own, else the post's, else `[market] default_currency`.
    #[test]
    fn the_currency_precedence_is_offer_then_post_then_server_default() {
        assert_eq!(
            resolve_offer_currency(Some("GBP"), Some("USD"), Some("EUR")),
            Ok("GBP".to_string()),
            "the offer's own currency wins outright"
        );
        assert_eq!(
            resolve_offer_currency(None, Some("USD"), Some("EUR")),
            Ok("USD".to_string()),
            "a counter with no currency inherits the listing's"
        );
        assert_eq!(
            resolve_offer_currency(None, None, Some("EUR")),
            Ok("EUR".to_string()),
            "the server default is the last resort, not the first"
        );
    }

    /// The fourth step does not exist. Inventing one would denominate somebody's deal in a unit
    /// neither party named, and look from the outside exactly like a deliberate choice.
    #[test]
    fn with_no_currency_anywhere_the_offer_is_refused_rather_than_priced_in_a_guess() {
        let err = resolve_offer_currency(None, None, None)
            .expect_err("an amount with no currency must not be stored");
        assert!(err.contains("currency"), "the 400 must name what is missing, got: {err}");
        assert!(
            err.contains("[market] default_currency"),
            "the 400 must name the key an operator would set, got: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // M2.3 / M2.4 — the deal lifecycle
    // -----------------------------------------------------------------------

    /// The happy walk the card describes, as a sequence of transitions rather than three
    /// independent assertions: `proposed -> accepted -> completed`.
    #[test]
    fn the_offer_counter_accept_complete_walk_is_legal_at_every_step() {
        check_accept_allowed(MatchStatus::Proposed).expect("a proposed thread may be accepted");
        check_transition(MatchStatus::Proposed, MatchStatus::Accepted).expect("accept");
        check_transition(MatchStatus::Accepted, MatchStatus::Completed).expect("complete");
    }

    /// The whole matrix, so a transition nobody thought about is decided here rather than by
    /// whichever `UPDATE` happens to run.
    #[test]
    fn the_transition_matrix_allows_exactly_five_moves() {
        use MatchStatus::*;

        let legal = [
            (Proposed, Proposed),
            (Proposed, Accepted),
            (Proposed, Withdrawn),
            (Accepted, Completed),
            (Accepted, Withdrawn),
        ];

        for from in MatchStatus::ALL {
            for to in MatchStatus::ALL {
                let expected = legal.contains(&(*from, *to));
                let actual = check_transition(*from, *to).is_ok();
                assert_eq!(
                    actual, expected,
                    "{from} -> {to} must be {}",
                    if expected { "allowed" } else { "refused" }
                );
            }
        }
    }

    /// Whatever the refusal says, it has to say what the thread is *now*: that is the one fact the
    /// caller does not have, and without it a 409 tells them only that they were wrong.
    #[test]
    fn every_refusal_names_the_status_the_conversation_is_actually_in() {
        for from in MatchStatus::ALL {
            for to in MatchStatus::ALL {
                if let Err(why) = check_transition(*from, *to) {
                    assert!(
                        why.contains(from.as_str()),
                        "{from} -> {to} refused without naming the current status: {why}"
                    );
                }
            }
        }
    }

    /// The two the card calls out by name.
    #[test]
    fn the_two_illegal_moves_the_card_names_are_409s_that_name_the_current_status() {
        let err = check_transition(MatchStatus::Proposed, MatchStatus::Completed)
            .expect_err("a deal cannot be completed before it is accepted");
        assert!(err.contains("proposed"), "got: {err}");

        let err = check_transition(MatchStatus::Completed, MatchStatus::Accepted)
            .expect_err("a completed deal cannot be re-accepted");
        assert!(err.contains("completed"), "got: {err}");
    }

    /// Going back to `proposed` would leave `agreed_price_cents` and `currency` set on a thread
    /// that no longer has an agreement — a price nobody agreed to, on a live negotiation.
    #[test]
    fn an_accepted_thread_cannot_be_walked_back_to_proposed() {
        check_transition(MatchStatus::Accepted, MatchStatus::Proposed)
            .expect_err("un-accepting must not be a status change");
    }

    /// Withdraw stays open to either participant from either live state, and closes from neither
    /// terminal one.
    #[test]
    fn withdraw_is_legal_while_a_deal_is_live_and_not_after_it_is_over() {
        check_transition(MatchStatus::Proposed, MatchStatus::Withdrawn).expect("from proposed");
        check_transition(MatchStatus::Accepted, MatchStatus::Withdrawn).expect("from accepted");
        check_transition(MatchStatus::Completed, MatchStatus::Withdrawn)
            .expect_err("a completed deal is not withdrawable");
        check_transition(MatchStatus::Withdrawn, MatchStatus::Withdrawn)
            .expect_err("withdrawing twice is a conflict, not a no-op");
    }

    /// M2.3: accepting is a `proposed -> accepted` move, so a thread that is already accepted,
    /// already completed, or withdrawn refuses it — and says which.
    #[test]
    fn an_accept_is_refused_on_any_thread_that_is_no_longer_proposed() {
        check_accept_allowed(MatchStatus::Proposed).expect("the only state an accept is legal in");

        for current in [MatchStatus::Accepted, MatchStatus::Completed, MatchStatus::Withdrawn] {
            let err = check_accept_allowed(current)
                .expect_err("an accept on a thread that is no longer proposed must be refused");
            assert!(
                err.contains(current.as_str()),
                "the 409 must say the thread is already '{current}', got: {err}"
            );
        }
    }

    /// M2.3: the counterparty accepts. Accepting your own number is not an agreement, it is one
    /// person writing both halves of the deal.
    #[test]
    fn you_cannot_accept_your_own_offer() {
        let me = Uuid::now_v7();
        let them = Uuid::now_v7();

        check_accept_actor(Some(them), me).expect("accepting the other party's offer is the point");

        let err = check_accept_actor(Some(me), me).expect_err("a self-accept must be refused");
        assert_eq!(err, SELF_ACCEPT);
        assert!(
            err.contains("your own offer"),
            "the 409 must say what was wrong with it, got: {err}"
        );
    }

    /// An accept with nothing to accept is not an agreement either — and would otherwise write an
    /// `agreed_price_cents` that one party picked unilaterally.
    #[test]
    fn an_accept_before_any_offer_is_refused() {
        let err = check_accept_actor(None, Uuid::now_v7())
            .expect_err("there is nothing to accept on an empty thread");
        assert_eq!(err, NOTHING_TO_ACCEPT);
    }

    // -----------------------------------------------------------------------
    // M2 — the status parameter, and the append-only guarantee
    // -----------------------------------------------------------------------

    #[test]
    fn the_status_patch_takes_the_four_values_the_check_allows() {
        for status in MatchStatus::ALL {
            assert_eq!(parse_status(status.as_str()), Ok(*status));
        }
        assert_eq!(parse_status("  completed  "), Ok(MatchStatus::Completed));

        for bad in ["", "   ", "sold", "Accepted", "done"] {
            let err = parse_status(bad).expect_err("an unknown status must be a 400");
            assert!(err.contains("status"), "the 400 must name the field, got: {err}");
            for status in MatchStatus::ALL {
                assert!(err.contains(status.as_str()), "the 400 must list {status}, got: {err}");
            }
        }
    }

    /// M2.1: "Rows are append-only: never update or delete one."
    ///
    /// The runtime gate proves the trail comes back in order with every step in it; what it cannot
    /// prove is that no code path anywhere rewrites a row, because a path nobody exercised is a
    /// path no curl reaches. This reads the only module that may touch the table and asserts the
    /// two statements that would break the guarantee are not in it.
    #[test]
    fn no_code_updates_or_deletes_an_offer_row() {
        const DB_CONVERSATIONS: &str = include_str!("../db/conversations.rs");

        let statements: Vec<&str> = DB_CONVERSATIONS
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with("//"))
            .filter(|line| {
                let upper = line.to_uppercase();
                (upper.contains("UPDATE ") || upper.contains("DELETE "))
                    && upper.contains("MATCH_OFFERS")
            })
            .collect();

        assert!(
            statements.is_empty(),
            "match_offers is append-only, but this module rewrites it: {statements:?}"
        );
        assert!(
            DB_CONVERSATIONS.contains("INSERT INTO match_offers"),
            "the fixture path must be wrong: no insert into match_offers was found at all"
        );
    }

    /// The trail is read oldest-first with a deterministic tie-break, or "every step in order" is
    /// only true of the runs where two rows did not land in the same microsecond.
    #[test]
    fn the_offer_list_is_ordered_oldest_first_and_breaks_ties_deterministically() {
        const DB_CONVERSATIONS: &str = include_str!("../db/conversations.rs");

        assert!(
            DB_CONVERSATIONS.contains("ORDER BY created_at ASC, id ASC"),
            "list_offers must order by created_at and then by id"
        );
    }
}
