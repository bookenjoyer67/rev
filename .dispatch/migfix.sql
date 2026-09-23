-- A2a hub item 7: record migration 1 as applied (schema was loaded directly, not by sqlx).
insert into _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
values (1, 'schema', now(), true, decode('c8e5870153bbe4688c7bb06e39eb4bdb0f2bf32b98ebf161b383e872fc1accfbea66feb05f630fde692193b69df8452e', 'hex'), 0)
on conflict (version) do nothing;
select version, description, success from _sqlx_migrations order by version;
