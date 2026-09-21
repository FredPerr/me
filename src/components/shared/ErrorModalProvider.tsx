import { useDisclosure } from "@mantine/hooks";
import { createContext, type ReactNode, useCallback, useContext, useState } from "react";
import { ErrorModal, type ErrorModalContent } from "./ErrorModal";

type ErrorModalContextValue = {
	showError: (content: ErrorModalContent) => void;
};

const ErrorModalContext = createContext<ErrorModalContextValue | null>(null);

export function ErrorModalProvider({ children }: { children: ReactNode }) {
	const [content, setContent] = useState<ErrorModalContent | null>(null);
	const [opened, { open, close }] = useDisclosure(false);

	const showError = useCallback(
		(next: ErrorModalContent) => {
			setContent(next);
			open();
		},
		[open],
	);

	return (
		<ErrorModalContext.Provider value={{ showError }}>
			{children}
			<ErrorModal opened={opened} onClose={close} content={content} />
		</ErrorModalContext.Provider>
	);
}

export function useErrorModal(): ErrorModalContextValue {
	const context = useContext(ErrorModalContext);
	if (!context) {
		throw new Error("useErrorModal must be used within an ErrorModalProvider");
	}
	return context;
}
