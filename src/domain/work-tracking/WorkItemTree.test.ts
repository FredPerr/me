import { describe, expect, it } from "vitest";
import type { WorkItemId, WorkProjectId } from "./identifiers";
import type { WorkItem } from "./WorkItem";
import { WorkItemPriority } from "./WorkItemPriority";
import { WorkItemStatus } from "./WorkItemStatus";
import { buildWorkItemTree, type WorkItemNode } from "./WorkItemTree";

function buildItem(id: string, parentId?: string): WorkItem {
	return {
		id: id as WorkItemId,
		projectId: "1" as WorkProjectId,
		...(parentId !== undefined && { parentId: parentId as WorkItemId }),
		title: `Task ${id}`,
		status: WorkItemStatus.Todo,
		rawStatus: "new",
		priority: WorkItemPriority.None,
		assignees: [],
		labels: [],
		url: `https://acme.teamwork.com/app/tasks/${id}`,
	};
}

type TreeShape = { id: string; children: TreeShape[] };

function shape(nodes: readonly WorkItemNode[]): TreeShape[] {
	return nodes.map((node) => ({ id: node.item.id, children: shape(node.children) }));
}

describe("buildWorkItemTree", () => {
	it("nests subtasks under their parent and keeps the input order", () => {
		const tree = buildWorkItemTree([
			buildItem("1"),
			buildItem("2", "1"),
			buildItem("3"),
			buildItem("4", "1"),
			buildItem("5", "2"),
		]);

		expect(shape(tree)).toEqual([
			{
				id: "1",
				children: [
					{ id: "2", children: [{ id: "5", children: [] }] },
					{ id: "4", children: [] },
				],
			},
			{ id: "3", children: [] },
		]);
	});

	it("attaches a subtask listed before its parent", () => {
		const tree = buildWorkItemTree([buildItem("2", "1"), buildItem("1")]);

		expect(shape(tree)).toEqual([{ id: "1", children: [{ id: "2", children: [] }] }]);
	});

	it("keeps an orphan subtask as a root with its parentId", () => {
		const tree = buildWorkItemTree([buildItem("1"), buildItem("2", "99")]);

		expect(shape(tree)).toEqual([
			{ id: "1", children: [] },
			{ id: "2", children: [] },
		]);
		expect(tree[1].item.parentId).toBe("99");
	});

	it("treats a self-parent as a root", () => {
		const tree = buildWorkItemTree([buildItem("1", "1")]);

		expect(shape(tree)).toEqual([{ id: "1", children: [] }]);
	});

	it("breaks a parent cycle without dropping items", () => {
		const tree = buildWorkItemTree([buildItem("1", "3"), buildItem("2", "1"), buildItem("3", "2")]);

		expect(shape(tree)).toEqual([
			{
				id: "3",
				children: [{ id: "1", children: [{ id: "2", children: [] }] }],
			},
		]);
	});

	it("returns no roots for no items", () => {
		expect(buildWorkItemTree([])).toEqual([]);
	});
});
