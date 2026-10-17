/** Every work tracking query starts with this key, so one invalidation refetches them all. */
const WORK_TRACKING_QUERY_KEY = ["workTracking"] as const;

export const CONNECTIONS_QUERY_KEY = [...WORK_TRACKING_QUERY_KEY, "connections"] as const;
