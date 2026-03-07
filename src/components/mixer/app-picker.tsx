import { ChevronDownIcon } from 'lucide-react'
import { Checkbox } from '~/components/ui/checkbox'
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '~/components/ui/popover'
import type { NodeInfo } from '~/lib/types'
import { cn } from '~/lib/utils'

interface AppPickerProps {
    /** All current PipeWire nodes — component filters to audio-producing streams. */
    nodes: NodeInfo[]
    /** Currently selected app names (process names from the backend). */
    selected: string[]
    onChange: (apps: string[]) => void
}

export function AppPicker({ nodes, selected, onChange }: AppPickerProps) {
    // Only show application audio streams, excluding our own virtual sinks.
    const appNodes = nodes.filter(
        (n) =>
            !n.isAmplitudeVirtual && n.mediaClass?.type === 'streamOutputAudio',
    )

    const toggleApp = (name: string) => {
        if (selected.includes(name)) {
            onChange(selected.filter((a) => a !== name))
        } else {
            onChange([...selected, name])
        }
    }

    const label =
        selected.length === 0
            ? 'Select apps'
            : selected.length === 1
              ? selected[0]
              : `${selected.length} apps`

    return (
        <Popover>
            <PopoverTrigger asChild>
                <button
                    type="button"
                    className={cn(
                        'border-input bg-input/30 flex h-7 w-full items-center justify-between gap-1 rounded-lg border px-2 text-[10px] font-medium transition-colors',
                        'hover:bg-input/50',
                        'focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] outline-none',
                        selected.length === 0 && 'text-muted-foreground',
                    )}
                >
                    <span className="truncate">{label}</span>
                    <ChevronDownIcon className="size-3 shrink-0 text-muted-foreground" />
                </button>
            </PopoverTrigger>
            <PopoverContent align="start" className="w-48 gap-0 p-1">
                {appNodes.length === 0 ? (
                    <p className="px-2.5 py-1.5 text-sm text-muted-foreground">
                        No apps playing audio
                    </p>
                ) : (
                    appNodes.map((node) => {
                        const displayName =
                            node.appName ?? node.description ?? node.name
                        const isChecked = selected.includes(displayName)
                        return (
                            <button
                                key={node.id}
                                type="button"
                                onClick={() => toggleApp(displayName)}
                                className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-sm transition-colors hover:bg-accent"
                            >
                                <Checkbox
                                    checked={isChecked}
                                    tabIndex={-1}
                                    className="pointer-events-none"
                                />
                                <span>{displayName}</span>
                            </button>
                        )
                    })
                )}
            </PopoverContent>
        </Popover>
    )
}
