DO $$
DECLARE
    current_date DATE := CURRENT_DATE;
    future_date DATE;
    partition_name TEXT;
    table_prefix TEXT := '{0}';
    days_ahead INTEGER := {1};
    index_name TEXT;
BEGIN
    FOR i IN 0..days_ahead LOOP
        future_date := current_date + (i * INTERVAL '1 day');
        partition_name := table_prefix || '_' || TO_CHAR(future_date, 'YYYYMMDD');
        
        EXECUTE 'CREATE TABLE IF NOT EXISTS ' || partition_name || 
                ' PARTITION OF ' || table_prefix || 
                ' FOR VALUES FROM (''' || future_date || 
                ''') TO (''' || (future_date + INTERVAL '1 day') || ''')'; 
        
        -- 파티션별 인덱스 생성
        IF table_prefix = 'request_logs' THEN
            -- request_logs 테이블의 파티션별 인덱스 생성
            index_name := partition_name || '_host_idx';
            EXECUTE 'CREATE INDEX IF NOT EXISTS ' || index_name || ' ON ' || partition_name || '(host)';
            
            index_name := partition_name || '_timestamp_idx';
            EXECUTE 'CREATE INDEX IF NOT EXISTS ' || index_name || ' ON ' || partition_name || '(timestamp)';
            
            index_name := partition_name || '_is_rejected_idx';
            EXECUTE 'CREATE INDEX IF NOT EXISTS ' || index_name || ' ON ' || partition_name || '(is_rejected)';
            
            index_name := partition_name || '_is_tls_idx';
            EXECUTE 'CREATE INDEX IF NOT EXISTS ' || index_name || ' ON ' || partition_name || '(is_tls)';
            
            index_name := partition_name || '_client_ip_idx';
            EXECUTE 'CREATE INDEX IF NOT EXISTS ' || index_name || ' ON ' || partition_name || '(client_ip)';
            
            index_name := partition_name || '_target_ip_idx';
            EXECUTE 'CREATE INDEX IF NOT EXISTS ' || index_name || ' ON ' || partition_name || '(target_ip)';
        ELSIF table_prefix = 'response_logs' THEN
            -- response_logs 테이블의 파티션별 인덱스 생성
            index_name := partition_name || '_session_id_idx';
            EXECUTE 'CREATE INDEX IF NOT EXISTS ' || index_name || ' ON ' || partition_name || '(session_id)';
            
            index_name := partition_name || '_timestamp_idx';
            EXECUTE 'CREATE INDEX IF NOT EXISTS ' || index_name || ' ON ' || partition_name || '(timestamp)';
            
            index_name := partition_name || '_status_code_idx';
            EXECUTE 'CREATE INDEX IF NOT EXISTS ' || index_name || ' ON ' || partition_name || '(status_code)';
        ELSIF table_prefix = 'proxy_stats' THEN
            -- proxy_stats 테이블의 파티션별 인덱스 생성
            index_name := partition_name || '_timestamp_idx';
            EXECUTE 'CREATE INDEX IF NOT EXISTS ' || index_name || ' ON ' || partition_name || '(timestamp)';
        ELSIF table_prefix = 'proxy_stats_hourly' THEN
            -- proxy_stats_hourly 테이블의 파티션별 인덱스 생성
            index_name := partition_name || '_timestamp_idx';
            EXECUTE 'CREATE INDEX IF NOT EXISTS ' || index_name || ' ON ' || partition_name || '(timestamp)';
        END IF;
        
        RAISE NOTICE 'Created partition and indexes: %', partition_name;
    END LOOP;
END $$;