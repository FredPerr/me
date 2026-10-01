import type { RemoteProjectLinkRepository } from "@/application/work-tracking/RemoteProjectLinkRepository";
import type { WorkTrackingGateway } from "@/domain/work-tracking/WorkTrackingGateway";
import { ProjectDirectoryRemoteProjectLinkRepository } from "./ProjectDirectoryRemoteProjectLinkRepository";
import { TauriWorkTrackingGateway } from "./TauriWorkTrackingGateway";

export const workTrackingGateway: WorkTrackingGateway = new TauriWorkTrackingGateway();

export const remoteProjectLinkRepository: RemoteProjectLinkRepository =
	new ProjectDirectoryRemoteProjectLinkRepository();
