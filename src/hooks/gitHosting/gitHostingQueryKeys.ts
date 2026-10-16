/** Every Git hosting query starts with this key, so one invalidation refetches them all. */
export const GIT_HOSTING_QUERY_KEY = ["gitHosting"] as const;

export const HOSTED_ACCOUNTS_QUERY_KEY = [...GIT_HOSTING_QUERY_KEY, "accounts"] as const;
