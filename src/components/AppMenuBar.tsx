/**
 * AppMenuBar Component
 *
 * Top-level application menu bar providing File, Edit, Debug, and View menus.
 * Styled to feel like a native desktop application menu.
 */

import {
    Menubar,
    MenubarContent,
    MenubarItem,
    MenubarMenu,
    MenubarSeparator,
    MenubarShortcut,
    MenubarSub,
    MenubarSubContent,
    MenubarSubTrigger,
    MenubarTrigger,
} from "./ui/menubar";

interface AppMenuBarProps {
    onStep: () => void;
    onRun: () => void;
    onReset: () => void;
    onRefresh: () => void;
    onAssembleAndRun?: () => void;
    onLoadExample?: (name: string) => void;
    onSwitchTab?: (tab: string) => void;
    isRunning: boolean;
}

/** Desktop-style application menu bar */
export function AppMenuBar({
    onStep,
    onRun,
    onReset,
    onRefresh,
    onAssembleAndRun,
    onLoadExample,
    onSwitchTab,
    isRunning,
}: AppMenuBarProps) {
    return (
        <div
            data-no-select
            className="shrink-0 border-b border-border bg-background/80 backdrop-blur-sm"
        >
            <Menubar className="h-8 border-none rounded-none bg-transparent shadow-none px-1">
                <MenubarMenu>
                    <MenubarTrigger className="text-xs px-2.5 py-1 font-normal data-[state=open]:bg-accent">
                        File
                    </MenubarTrigger>
                    <MenubarContent align="start" className="min-w-[180px]">
                        <MenubarItem disabled>
                            Open ROM...
                            <MenubarShortcut>Ctrl+O</MenubarShortcut>
                        </MenubarItem>
                        <MenubarItem disabled>
                            Load Assembly...
                            <MenubarShortcut>Ctrl+L</MenubarShortcut>
                        </MenubarItem>
                        <MenubarSeparator />
                        {onLoadExample && (
                            <>
                                <MenubarSub>
                                    <MenubarSubTrigger>Examples</MenubarSubTrigger>
                                    <MenubarSubContent>
                                        <MenubarItem onSelect={() => onLoadExample("hello")}>
                                            Hello World
                                        </MenubarItem>
                                        <MenubarItem onSelect={() => onLoadExample("chars")}>
                                            Print Characters
                                        </MenubarItem>
                                        <MenubarItem onSelect={() => onLoadExample("count")}>
                                            Count to 10
                                        </MenubarItem>
                                        <MenubarItem onSelect={() => onLoadExample("memory")}>
                                            Memory Operations
                                        </MenubarItem>
                                    </MenubarSubContent>
                                </MenubarSub>
                                <MenubarSeparator />
                            </>
                        )}
                        <MenubarItem disabled>
                            Export Memory Dump...
                        </MenubarItem>
                        <MenubarSeparator />
                        <MenubarItem>
                            Exit
                            <MenubarShortcut>Alt+F4</MenubarShortcut>
                        </MenubarItem>
                    </MenubarContent>
                </MenubarMenu>

                <MenubarMenu>
                    <MenubarTrigger className="text-xs px-2.5 py-1 font-normal data-[state=open]:bg-accent">
                        Edit
                    </MenubarTrigger>
                    <MenubarContent align="start" className="min-w-[180px]">
                        <MenubarItem disabled>
                            Go to Address...
                            <MenubarShortcut>Ctrl+G</MenubarShortcut>
                        </MenubarItem>
                        <MenubarSeparator />
                        <MenubarItem disabled>
                            Write Byte...
                        </MenubarItem>
                        <MenubarItem disabled>
                            Set Register...
                        </MenubarItem>
                    </MenubarContent>
                </MenubarMenu>

                <MenubarMenu>
                    <MenubarTrigger className="text-xs px-2.5 py-1 font-normal data-[state=open]:bg-accent">
                        Debug
                    </MenubarTrigger>
                    <MenubarContent align="start" className="min-w-[200px]">
                        {onAssembleAndRun && (
                            <>
                                <MenubarItem disabled={isRunning} onSelect={onAssembleAndRun}>
                                    Assemble &amp; Run
                                    <MenubarShortcut>F5</MenubarShortcut>
                                </MenubarItem>
                                <MenubarSeparator />
                            </>
                        )}
                        <MenubarItem disabled={isRunning} onSelect={onStep}>
                            Step Instruction
                            <MenubarShortcut>F10</MenubarShortcut>
                        </MenubarItem>
                        <MenubarItem disabled={isRunning} onSelect={onRun}>
                            Continue
                            <MenubarShortcut>F8</MenubarShortcut>
                        </MenubarItem>
                        <MenubarSeparator />
                        <MenubarItem disabled={isRunning} onSelect={onReset}>
                            Reset CPU
                            <MenubarShortcut>F6</MenubarShortcut>
                        </MenubarItem>
                        <MenubarItem onSelect={onRefresh}>
                            Refresh State
                            <MenubarShortcut>F9</MenubarShortcut>
                        </MenubarItem>
                    </MenubarContent>
                </MenubarMenu>

                <MenubarMenu>
                    <MenubarTrigger className="text-xs px-2.5 py-1 font-normal data-[state=open]:bg-accent">
                        View
                    </MenubarTrigger>
                    <MenubarContent align="start" className="min-w-[180px]">
                        <MenubarItem onSelect={() => onSwitchTab?.("registers")}>
                            Registers
                            <MenubarShortcut>Ctrl+⇧+1</MenubarShortcut>
                        </MenubarItem>
                        <MenubarItem onSelect={() => onSwitchTab?.("memory")}>
                            Memory
                            <MenubarShortcut>Ctrl+⇧+2</MenubarShortcut>
                        </MenubarItem>
                        <MenubarItem onSelect={() => onSwitchTab?.("uart")}>
                            UART Terminal
                            <MenubarShortcut>Ctrl+⇧+3</MenubarShortcut>
                        </MenubarItem>
                        <MenubarSeparator />
                        <MenubarItem disabled>
                            Reset Layout
                        </MenubarItem>
                    </MenubarContent>
                </MenubarMenu>

                <MenubarMenu>
                    <MenubarTrigger className="text-xs px-2.5 py-1 font-normal data-[state=open]:bg-accent">
                        Help
                    </MenubarTrigger>
                    <MenubarContent align="start" className="min-w-[180px]">
                        <MenubarItem disabled>
                            M68K Reference
                        </MenubarItem>
                        <MenubarSub>
                            <MenubarSubTrigger>Keyboard Shortcuts</MenubarSubTrigger>
                            <MenubarSubContent className="min-w-[220px]">
                                <MenubarItem disabled className="text-xs">
                                    Assemble &amp; Run <MenubarShortcut>F5</MenubarShortcut>
                                </MenubarItem>
                                <MenubarItem disabled className="text-xs">
                                    Continue <MenubarShortcut>F8</MenubarShortcut>
                                </MenubarItem>
                                <MenubarItem disabled className="text-xs">
                                    Step Instruction <MenubarShortcut>F10</MenubarShortcut>
                                </MenubarItem>
                                <MenubarItem disabled className="text-xs">
                                    Reset CPU <MenubarShortcut>F6</MenubarShortcut>
                                </MenubarItem>
                                <MenubarItem disabled className="text-xs">
                                    Refresh State <MenubarShortcut>F9</MenubarShortcut>
                                </MenubarItem>
                                <MenubarSeparator />
                                <MenubarItem disabled className="text-xs">
                                    Registers <MenubarShortcut>Ctrl+⇧+1</MenubarShortcut>
                                </MenubarItem>
                                <MenubarItem disabled className="text-xs">
                                    Memory <MenubarShortcut>Ctrl+⇧+2</MenubarShortcut>
                                </MenubarItem>
                                <MenubarItem disabled className="text-xs">
                                    UART <MenubarShortcut>Ctrl+⇧+3</MenubarShortcut>
                                </MenubarItem>
                            </MenubarSubContent>
                        </MenubarSub>
                        <MenubarSeparator />
                        <MenubarItem disabled className="text-[10px] text-muted-foreground">
                            Flux32 v1.0 — M68K Emulator
                        </MenubarItem>
                    </MenubarContent>
                </MenubarMenu>
            </Menubar>
        </div>
    );
}
