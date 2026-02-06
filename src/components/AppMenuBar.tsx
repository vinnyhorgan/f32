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
    MenubarTrigger,
} from "./ui/menubar";

interface AppMenuBarProps {
    onStep: () => void;
    onRun: () => void;
    onReset: () => void;
    onRefresh: () => void;
    isRunning: boolean;
}

/** Desktop-style application menu bar */
export function AppMenuBar({
    onStep,
    onRun,
    onReset,
    onRefresh,
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
                        <MenubarItem>
                            Open ROM...
                            <MenubarShortcut>Ctrl+O</MenubarShortcut>
                        </MenubarItem>
                        <MenubarItem>
                            Load Assembly...
                            <MenubarShortcut>Ctrl+L</MenubarShortcut>
                        </MenubarItem>
                        <MenubarSeparator />
                        <MenubarItem>
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
                        <MenubarItem>
                            Go to Address...
                            <MenubarShortcut>Ctrl+G</MenubarShortcut>
                        </MenubarItem>
                        <MenubarSeparator />
                        <MenubarItem>
                            Write Byte...
                        </MenubarItem>
                        <MenubarItem>
                            Set Register...
                        </MenubarItem>
                    </MenubarContent>
                </MenubarMenu>

                <MenubarMenu>
                    <MenubarTrigger className="text-xs px-2.5 py-1 font-normal data-[state=open]:bg-accent">
                        Debug
                    </MenubarTrigger>
                    <MenubarContent align="start" className="min-w-[200px]">
                        <MenubarItem disabled={isRunning} onSelect={onStep}>
                            Step Instruction
                            <MenubarShortcut>F10</MenubarShortcut>
                        </MenubarItem>
                        <MenubarItem disabled={isRunning} onSelect={onRun}>
                            Run
                            <MenubarShortcut>F5</MenubarShortcut>
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
                        <MenubarItem>
                            Registers
                        </MenubarItem>
                        <MenubarItem>
                            Memory
                        </MenubarItem>
                        <MenubarSeparator />
                        <MenubarItem>
                            Reset Layout
                        </MenubarItem>
                    </MenubarContent>
                </MenubarMenu>

                <MenubarMenu>
                    <MenubarTrigger className="text-xs px-2.5 py-1 font-normal data-[state=open]:bg-accent">
                        Help
                    </MenubarTrigger>
                    <MenubarContent align="start" className="min-w-[180px]">
                        <MenubarItem>
                            M68K Reference
                        </MenubarItem>
                        <MenubarItem>
                            Keyboard Shortcuts
                        </MenubarItem>
                        <MenubarSeparator />
                        <MenubarItem>
                            About Flux32
                        </MenubarItem>
                    </MenubarContent>
                </MenubarMenu>
            </Menubar>
        </div>
    );
}
