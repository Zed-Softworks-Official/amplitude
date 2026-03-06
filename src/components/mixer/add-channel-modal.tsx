import { useRef, useState } from 'react'
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from '~/components/ui/dialog'
import { cn } from '~/lib/utils'
import { addChannel } from '~/lib/tauri-api'
import { ChannelIcon } from './channel-icon'

const ADDABLE_PRESETS = [
    { name: 'System' },
    { name: 'Browser' },
    { name: 'VC' },
    { name: 'Game' },
    { name: 'Music' },
] as const

interface AddChannelModalProps {
    open: boolean
    onOpenChange: (open: boolean) => void
    existingChannelNames: string[]
}

export function AddChannelModal({
    open,
    onOpenChange,
    existingChannelNames,
}: AddChannelModalProps) {
    const lowerExisting = existingChannelNames.map((n) => n.toLowerCase())
    const [isPending, setIsPending] = useState(false)
    const [error, setError] = useState<string | null>(null)
    const [customName, setCustomName] = useState('')
    const inputRef = useRef<HTMLInputElement>(null)

    const handleAdd = (name: string) => {
        const trimmed = name.trim()
        if (!trimmed) return

        setIsPending(true)
        setError(null)
        addChannel(trimmed)
            .then(() => {
                setCustomName('')
                onOpenChange(false)
            })
            .catch((err: unknown) => {
                console.error(err)
                setError(
                    err instanceof Error
                        ? err.message
                        : typeof err === 'string'
                          ? err
                          : 'Failed to add channel. Please try again.',
                )
            })
            .finally(() => {
                setIsPending(false)
            })
    }

    const handleOpenChange = (nextOpen: boolean) => {
        if (!isPending) {
            setError(null)
            setCustomName('')
            onOpenChange(nextOpen)
        }
    }

    const customNameLower = customName.trim().toLowerCase()
    const customIsUsed =
        customNameLower.length > 0 && lowerExisting.includes(customNameLower)
    const customIsReserved = customNameLower === 'mic'
    const customValid =
        customName.trim().length > 0 && !customIsUsed && !customIsReserved

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogContent className="sm:max-w-sm">
                <DialogHeader>
                    <DialogTitle>Add Channel</DialogTitle>
                    <DialogDescription>
                        Choose a preset or enter a custom name.
                    </DialogDescription>
                </DialogHeader>

                {error && (
                    <p className="text-sm text-destructive">{error}</p>
                )}

                <div className="grid grid-cols-2 gap-2">
                    {ADDABLE_PRESETS.map(({ name }) => {
                        const isUsed = lowerExisting.includes(name.toLowerCase())

                        return (
                            <button
                                key={name}
                                type="button"
                                disabled={isUsed || isPending}
                                onClick={() => handleAdd(name)}
                                className={cn(
                                    'flex items-center gap-3 rounded-xl border border-border bg-card p-3 text-left text-sm font-medium transition-colors',
                                    'hover:border-accent/30 hover:bg-accent/5',
                                    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
                                    (isUsed || isPending) && 'pointer-events-none opacity-35',
                                )}
                            >
                                <div className="flex size-8 items-center justify-center rounded-lg bg-muted">
                                    <ChannelIcon
                                        name={name}
                                        className="size-4 text-muted-foreground"
                                    />
                                </div>
                                <div className="flex flex-col">
                                    <span className="text-foreground">
                                        {name}
                                    </span>
                                    {isUsed && (
                                        <span className="text-[10px] text-muted-foreground">
                                            Already added
                                        </span>
                                    )}
                                </div>
                            </button>
                        )
                    })}
                </div>

                <div className="mt-1 flex flex-col gap-1.5">
                    <p className="text-xs font-medium text-muted-foreground">
                        Custom
                    </p>
                    <form
                        onSubmit={(e) => {
                            e.preventDefault()
                            if (customValid && !isPending) handleAdd(customName)
                        }}
                        className="flex gap-2"
                    >
                        <input
                            ref={inputRef}
                            type="text"
                            placeholder="Channel name…"
                            value={customName}
                            onChange={(e) => {
                                setError(null)
                                setCustomName(e.target.value)
                            }}
                            disabled={isPending}
                            maxLength={32}
                            className={cn(
                                'flex-1 rounded-lg border border-border bg-card px-3 py-2 text-sm outline-none transition-colors',
                                'placeholder:text-muted-foreground',
                                'focus:border-ring focus:ring-1 focus:ring-ring',
                                isPending && 'opacity-50',
                            )}
                        />
                        <button
                            type="submit"
                            disabled={!customValid || isPending}
                            className={cn(
                                'rounded-lg border border-border bg-card px-3 py-2 text-sm font-medium transition-colors',
                                'hover:border-accent/30 hover:bg-accent/5',
                                'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
                                (!customValid || isPending) && 'pointer-events-none opacity-35',
                            )}
                        >
                            Add
                        </button>
                    </form>
                    {customIsUsed && (
                        <p className="text-[11px] text-muted-foreground">
                            A channel with that name already exists.
                        </p>
                    )}
                    {customIsReserved && (
                        <p className="text-[11px] text-muted-foreground">
                            "mic" is a reserved channel name.
                        </p>
                    )}
                </div>
            </DialogContent>
        </Dialog>
    )
}
