import type { InvalidInputReason } from "./InvalidInputReason";

/**
 * A remote project link that failed validation. Thrown only inside the link
 * value objects and caught by `RemoteProjectLinks.fromJSON`; it never crosses
 * the command boundary, so it is a plain domain error, not a `CommandError`.
 */
export class RemoteProjectLinkError extends Error {
	readonly reason: InvalidInputReason;

	constructor(reason: InvalidInputReason) {
		super("Invalid remote project link");
		this.name = "RemoteProjectLinkError";
		this.reason = reason;
	}
}
