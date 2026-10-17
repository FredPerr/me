import type { TFunction } from "i18next";
import type { WorkTrackingCommandError } from "@/bindings";
import { errorMessageKey } from "./errorMessageKey";

export function translateError(t: TFunction, error: WorkTrackingCommandError): string {
	const { key, values } = errorMessageKey(error);
	return t(key, values);
}
