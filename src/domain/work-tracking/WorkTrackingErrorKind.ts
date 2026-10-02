export enum WorkTrackingErrorKind {
	NotConfigured = "notConfigured",
	InvalidInput = "invalidInput",
	Unauthorized = "unauthorized",
	Forbidden = "forbidden",
	NotFound = "notFound",
	RateLimited = "rateLimited",
	Timeout = "timeout",
	Network = "network",
	ProviderError = "providerError",
	InvalidResponse = "invalidResponse",
	StorageError = "storageError",
}
