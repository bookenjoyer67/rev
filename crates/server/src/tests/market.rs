//! M1.5 — the marketplace foundation.
//!
//! Everything here is a unit test over the pure halves of M1: the `[market]` validator, the
//! `?scope=` union rule, the slug/label rules the admin editor enforces, the audit payload, and
//! every market filter on `/api/posts`. That split is deliberate — a test that needs a live
//! Postgres does not run under `cargo test --workspace`, so the behaviour that needs a database
//! (the actual 403, the actual `audit_events` row) belongs to the card's runtime curl gate, and
//! what belongs here is the logic those responses are computed from.

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
