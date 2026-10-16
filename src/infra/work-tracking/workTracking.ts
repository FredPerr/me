import type { WorkTrackingGateway } from "@/domain/work-tracking/WorkTrackingGateway";
import { TauriWorkTrackingGateway } from "./TauriWorkTrackingGateway";

export const workTrackingGateway: WorkTrackingGateway = new TauriWorkTrackingGateway();
