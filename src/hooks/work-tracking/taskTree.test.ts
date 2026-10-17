import { describe, expect, it } from "vitest";
import type { Task, TaskId } from "@/bindings";
import { buildTaskTree, type TaskNode } from "./taskTree";

function buildTask(id: string, parentId: string | null = null): Task {
	return {
		id: id as TaskId,
		projectId: "1" as Task["projectId"],
		groupId: null,
		parentId: parentId as TaskId | null,
		title: `Task ${id}`,
		description: null,
		status: "todo",
		rawStatus: "new",
		priority: "none",
		rawPriority: null,
		assignees: [],
		labels: [],
		dueDate: null,
		url: `https://acme.teamwork.com/app/tasks/${id}`,
		updatedAt: null,
	};
}

type TreeShape = { id: string; children: TreeShape[] };

function shape(nodes: readonly TaskNode[]): TreeShape[] {
	return nodes.map((node) => ({ id: node.task.id, children: shape(node.children) }));
}

describe("buildTaskTree", () => {
	it("nests subtasks under their parent and keeps the input order", () => {
		const tree = buildTaskTree([
			buildTask("1"),
			buildTask("2", "1"),
			buildTask("3"),
			buildTask("4", "1"),
			buildTask("5", "2"),
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
		const tree = buildTaskTree([buildTask("2", "1"), buildTask("1")]);

		expect(shape(tree)).toEqual([{ id: "1", children: [{ id: "2", children: [] }] }]);
	});

	it("keeps an orphan subtask as a root with its parentId", () => {
		const tree = buildTaskTree([buildTask("1"), buildTask("2", "99")]);

		expect(shape(tree)).toEqual([
			{ id: "1", children: [] },
			{ id: "2", children: [] },
		]);
		expect(tree[1].task.parentId).toBe("99");
	});

	it("treats a self-parent as a root", () => {
		const tree = buildTaskTree([buildTask("1", "1")]);

		expect(shape(tree)).toEqual([{ id: "1", children: [] }]);
	});

	it("breaks a parent cycle without dropping tasks", () => {
		const tree = buildTaskTree([buildTask("1", "3"), buildTask("2", "1"), buildTask("3", "2")]);

		expect(shape(tree)).toEqual([
			{
				id: "3",
				children: [{ id: "1", children: [{ id: "2", children: [] }] }],
			},
		]);
	});

	it("returns no roots for no tasks", () => {
		expect(buildTaskTree([])).toEqual([]);
	});
});
