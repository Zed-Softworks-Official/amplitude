import { cn } from '~/lib/utils'

const SEGMENT_COUNT = 16

function getSegmentColor(index: number, total: number) {
    const position = index / total
    if (position >= 0.85) return 'bg-red-500'
    if (position >= 0.7) return 'bg-orange-400'
    return 'bg-green-500'
}

export function Meter({
    value,
    className,
}: {
    value: number
    className?: string
}) {
    const clamped = Math.max(0, Math.min(100, value))
    const activeSegments = Math.round((clamped / 100) * SEGMENT_COUNT)

    return (
        <div
            className={cn(
                'flex h-full w-2.5 flex-col-reverse gap-[2px]',
                className,
            )}
        >
            {Array.from({ length: SEGMENT_COUNT }, (_, i) => {
                const isActive = i < activeSegments
                const color = getSegmentColor(i, SEGMENT_COUNT)

                return (
                    <div
                        key={`seg-${i}`}
                        className={cn(
                            'flex-1 rounded-[1px] transition-opacity duration-100',
                            isActive ? color : 'bg-muted',
                            isActive ? 'opacity-100' : 'opacity-40',
                        )}
                    />
                )
            })}
        </div>
    )
}
