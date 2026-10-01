import type { TFunction } from "i18next";
import type { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { errorMessageKey } from "./errorMessageKey";

export function translateError(t: TFunction, error: WorkTrackingError): string {
	const { key, values } = errorMessageKey(error);
	return t(key, values);
}
