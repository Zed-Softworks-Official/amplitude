import { ChevronDownIcon } from 'lucide-react'
import { Checkbox } from '~/components/ui/checkbox'
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '~/components/ui/popover'
import { cn } from '~/lib/utils'
import { APPLICATIONS } from './types'

interface AppPickerProps {
    selected: string[]
    onChange: (apps: string[]) => void
}

export function AppPicker({ selected, onChange }: AppPickerProps) {
    const toggleApp = (app: string) => {
        if (selected.includes(app)) {
            onChange(selected.filter((a) => a !== app))
        } else {
            onChange([...selected, app])
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
                {APPLICATIONS.map((app) => {
                    const isChecked = selected.includes(app)
                    return (
                        <button
                            key={app}
                            type="button"
                            onClick={() => toggleApp(app)}
                            className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-sm transition-colors hover:bg-accent"
                        >
                            <Checkbox
                                checked={isChecked}
                                tabIndex={-1}
                                className="pointer-events-none"
                            />
                            <span>{app}</span>
                        </button>
                    )
                })}
            </PopoverContent>
        </Popover>
    )
}
