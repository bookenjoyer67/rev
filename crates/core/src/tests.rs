#[cfg(test)]
mod tests {
    use crate::models::*;
    use serde_json;

    #[test]
    fn test_post_kind_serialization() {
        assert_eq!(
            serde_json::to_string(&PostKind::Resource).unwrap(),
            "\"resource\""
        );
        assert_eq!(
            serde_json::to_string(&PostKind::Need).unwrap(),
            "\"need\""
        );
        assert_eq!(
            serde_json::to_string(&PostKind::Offer).unwrap(),
            "\"offer\""
        );
    }

    #[test]
    fn test_post_kind_deserialization() {
        let k: PostKind = serde_json::from_str("\"resource\"").unwrap();
        assert_eq!(k, PostKind::Resource);
        let k: PostKind = serde_json::from_str("\"need\"").unwrap();
        assert_eq!(k, PostKind::Need);
        let k: PostKind = serde_json::from_str("\"offer\"").unwrap();
        assert_eq!(k, PostKind::Offer);
    }

    #[test]
    fn test_visibility_serialization() {
        assert_eq!(
            serde_json::to_string(&Visibility::Public).unwrap(),
            "\"public\""
        );
        assert_eq!(
            serde_json::to_string(&Visibility::Federated).unwrap(),
            "\"federated\""
        );
        assert_eq!(
            serde_json::to_string(&Visibility::Private).unwrap(),
            "\"private\""
        );
    }

    #[test]
    fn test_visibility_deserialization() {
        let v: Visibility = serde_json::from_str("\"public\"").unwrap();
        assert_eq!(v, Visibility::Public);
        let v: Visibility = serde_json::from_str("\"federated\"").unwrap();
        assert_eq!(v, Visibility::Federated);
        let v: Visibility = serde_json::from_str("\"private\"").unwrap();
        assert_eq!(v, Visibility::Private);
    }

    #[test]
    fn test_urgency_serialization_lowercase() {
        assert_eq!(
            serde_json::to_string(&Urgency::Critical).unwrap(),
            "\"critical\""
        );
        assert_eq!(
            serde_json::to_string(&Urgency::Low).unwrap(),
            "\"low\""
        );
    }

    #[test]
    fn test_post_status_serialization() {
        assert_eq!(
            serde_json::to_string(&PostStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&PostStatus::Fulfilled).unwrap(),
            "\"fulfilled\""
        );
        assert_eq!(
            serde_json::to_string(&PostStatus::Withdrawn).unwrap(),
            "\"withdrawn\""
        );
    }

    #[test]
    fn test_category_roundtrip() {
        let categories = vec![
            Category::Food, Category::Shelter, Category::Health,
            Category::Transport, Category::Education, Category::Labor,
            Category::Legal, Category::Other,
        ];
        for cat in categories {
            let json = serde_json::to_string(&cat).unwrap();
            let back: Category = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", cat), format!("{:?}", back));
        }
    }

    #[test]
    fn test_alliance_status_roundtrip() {
        let statuses = [
            AllianceStatus::Pending,
            AllianceStatus::Accepted,
            AllianceStatus::Rejected,
            AllianceStatus::Severed,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let back: AllianceStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", s), format!("{:?}", back));
        }
    }

    #[test]
    fn test_create_community_json() {
        let input = CreateCommunity {
            name: "Test Community".into(),
            slug: "test-community".into(),
            description: Some("A test".into()),
            location_name: Some("St. Louis".into()),
            location_lat: Some(38.6270),
            location_lon: Some(-90.1994),
            visibility: Some(Visibility::Public),
        };
        let json = serde_json::to_string(&input).unwrap();
        let back: CreateCommunity = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Test Community");
        assert_eq!(back.slug, "test-community");
        assert_eq!(back.location_lat, Some(38.6270));
    }

    #[test]
    fn test_community_map_secret_excluded_when_none() {
        let community = Community {
            id: uuid::Uuid::now_v7(),
            slug: "stl".into(),
            name: "St. Louis Mutual Aid".into(),
            description: None,
            location_name: None,
            location_lat: None,
            location_lon: None,
            visibility: Visibility::Federated,
            created_at: chrono::Utc::now(),
            map_community_id: None,
            map_secret_hex: None,
        };
        let json = serde_json::to_string(&community).unwrap();
        assert!(!json.contains("map_community_id"));
        assert!(!json.contains("map_secret_hex"));
    }

    #[test]
    fn test_invite_json_roundtrip() {
        let invite = Invite {
            code: "abc12345".into(),
            community_id: uuid::Uuid::now_v7(),
            created_by: uuid::Uuid::now_v7(),
            uses_remaining: Some(5),
            expires_at: None,
            created_at: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&invite).unwrap();
        let back: Invite = serde_json::from_str(&json).unwrap();
        assert_eq!(back.code, "abc12345");
        assert_eq!(back.uses_remaining, Some(5));
    }

    #[test]
    fn test_post_serialization_excludes_optional_fields_when_none() {
        let post = Post {
            id: uuid::Uuid::now_v7(),
            community_id: uuid::Uuid::now_v7(),
            author_id: uuid::Uuid::now_v7(),
            kind: PostKind::Offer,
            title: "Free bread".into(),
            body: Some("Fresh sourdough".into()),
            category: Category::Food,
            urgency: None,
            status: PostStatus::Active,
            quantity: None,
            location_name: None,
            location_lat: None,
            location_lon: None,
            visibility: Visibility::Public,
            expires_at: None,
            tags: vec![],
            images: vec![],
            contact_method: None,
            verified_by: None,
            verified_at: None,
            federated_id: None,
            origin_node: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&post).unwrap();
        assert!(json.contains("\"federated_id\":null"));
        assert!(json.contains("\"origin_node\":null"));
    }
}
