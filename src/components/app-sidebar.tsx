import { useForm } from "@tanstack/react-form"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenuItem,
} from "@/components/ui/sidebar"
import { Balloon, CalendarDays, ChevronsDown, ChevronsUp, Clock, Loader, MapPin, Play } from "lucide-react"
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group"
import {
  Field,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Button } from "@/components/ui/button"

import { Calendar } from "@/components/ui/calendar"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"

export interface PredictorFormValues {
  launchPosition: string
  launchLat: number
  launchLon: number
  launchDate: Date | undefined
  launchTime: string
  ascentRate: string
  descentRate: string
  burstAltitude: string
}

interface AppSidebarProps {
  onSubmit: (values: PredictorFormValues) => void | Promise<void>
  isLoading?: boolean
}

// 24時間形式 (例: 13:00 や 09:30) のバリデーション
const validateTime24h = (value: string) => {
  if (!value) return "時刻を入力してください"
  const regex = /^([01]?[0-9]|2[0-3]):[0-5][0-9]$/
  if (!regex.test(value)) return "24時間形式 (例: 13:00) で入力してください"
  return undefined
}

const validateLaunchPosition = (value: string) => {
  if (!value) return "放球位置を入力してください"
  const parts = value.split(",").map(s => s.trim())
  if (parts.length !== 2) return "'緯度,経度' の形式で入力してください"
  const lat = parseFloat(parts[0])
  const lon = parseFloat(parts[1])
  if (isNaN(lat) || isNaN(lon)) return "緯度・経度は数値で入力してください"
  if (lat < -90 || lat > 90) return "緯度は -90〜90 の範囲で入力してください"
  if (lon < -180 || lon > 180) return "経度は -180〜180 の範囲で入力してください"
  return undefined
}

const validatePositiveNumber = (value: string, label: string) => {
  if (!value) return `${label}を入力してください`
  const num = Number(value)
  if (isNaN(num)) return "数値で入力してください"
  if (num <= 0) return "0より大きい数値を入力してください"
  return undefined
}

export function AppSidebar({ onSubmit, isLoading = false }: AppSidebarProps) {
  const defaultValues: PredictorFormValues = {
    launchPosition: "",
    launchLat: 0,
    launchLon: 0,
    launchDate: undefined,
    launchTime: "",
    ascentRate: "",
    descentRate: "",
    burstAltitude: "",
  }

  const form = useForm({
    defaultValues,
    onSubmit: async ({ value }) => {
      const [latStr, lonStr] = value.launchPosition.split(",").map(s => s.trim())
      await onSubmit({
        ...value,
        launchLat: parseFloat(latStr),
        launchLon: parseFloat(lonStr),
      })
    },
  })

  return (
    <Sidebar variant="floating">
      <form
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
        className="flex flex-col h-full"
      >
        <SidebarHeader className="py-4 px-3 flex flex-row items-center gap-2">
          <Balloon />
          <h1 className="font-medium">Space Balloon Predictor</h1>
        </SidebarHeader>

        <SidebarContent className="p-4 *:list-none">
          <SidebarMenuItem className="space-y-6">
            {/* 放球位置 */}
            <form.Field
              name="launchPosition"
              validators={{
                onChange: ({ value }) => validateLaunchPosition(value),
              }}
              children={(field) => (
                <Field>
                  <FieldLabel className="text-xs" htmlFor={field.name}>放球位置</FieldLabel>
                  <InputGroup>
                    <InputGroupInput
                      id={field.name}
                      name={field.name}
                      value={field.state.value}
                      onBlur={field.handleBlur}
                      onChange={(e) => field.handleChange(e.target.value)}
                      type="text"
                      placeholder="位置を入力"
                    />
                    <InputGroupAddon align="inline-start">
                      <MapPin className="text-muted-foreground" />
                    </InputGroupAddon>
                  </InputGroup>
                  {field.state.meta.isTouched && field.state.meta.errors.length ? (
                    <p className="text-[11px] text-red-500 mt-1">{field.state.meta.errors.join(", ")}</p>
                  ) : null}
                </Field>
              )}
            />

            <FieldGroup className="flex flex-col gap-y-2">
              {/* 放球日 (デザイン統一版) */}
              <form.Field
                name="launchDate"
                validators={{
                  onChange: ({ value }) => (!value ? "日付を選択してください" : undefined),
                }}
                children={(field) => (
                  <Field className="flex-1">
                    <FieldLabel className="text-xs" htmlFor={field.name}>放球日</FieldLabel>
                    <Popover>
                      <PopoverTrigger asChild>
                        {/* ボタン全体をInputGroupと同じ見た目にするため、中身にInputGroupを配置 */}
                        <button type="button" className="w-full text-left outline-none block">
                          <InputGroup>
                            <InputGroupInput
                              id={field.name}
                              name={field.name}
                              value={
                                field.state.value
                                  ? field.state.value.toLocaleDateString("ja-JP", {
                                      year: "numeric",
                                      month: "2-digit",
                                      day: "2-digit",
                                    })
                                  : ""
                              }
                              placeholder="日付を選択"
                              readOnly
                              className="cursor-pointer placeholder:text-muted-foreground placeholder:text-sm"
                            />
                            <InputGroupAddon align="inline-start">
                              <CalendarDays className="text-muted-foreground" />
                            </InputGroupAddon>
                          </InputGroup>
                        </button>
                      </PopoverTrigger>
                      <PopoverContent className="w-auto p-0" align="start">
                        <Calendar
                          mode="single"
                          selected={field.state.value as Date | undefined}
                          onSelect={(date) => {
                            field.handleChange(date as Date)
                            field.handleBlur()
                          }}
                        />
                      </PopoverContent>
                    </Popover>
                    {field.state.meta.isTouched && field.state.meta.errors.length ? (
                      <p className="text-[11px] text-red-500 mt-1">{field.state.meta.errors.join(", ")}</p>
                    ) : null}
                  </Field>
                )}
              />

              {/* 放球時刻 (24時間形式テキスト入力) */}
              <form.Field
                name="launchTime"
                validators={{
                  onChange: ({ value }) => validateTime24h(value),
                }}
                children={(field) => (
                  <Field className="flex-1">
                    <FieldLabel className="text-xs" htmlFor={field.name}>放球時刻</FieldLabel>
                    <InputGroup>
                      <InputGroupInput
                        id={field.name}
                        name={field.name}
                        value={field.state.value}
                        onBlur={field.handleBlur}
                        onChange={(e) => field.handleChange(e.target.value)}
                        type="text"
                        placeholder="13:00"
                        className="w-full"
                      />
                      <InputGroupAddon align="inline-start">
                        <Clock className="text-muted-foreground" />
                      </InputGroupAddon>
                    </InputGroup>
                    {field.state.meta.isTouched && field.state.meta.errors.length ? (
                      <p className="text-[11px] text-red-500 mt-1">{field.state.meta.errors.join(", ")}</p>
                    ) : null}
                  </Field>
                )}
              />
            </FieldGroup>

            <FieldGroup className="flex flex-col gap-y-2">
              {/* 上昇速度 */}
              <form.Field
                name="ascentRate"
                validators={{
                  onChange: ({ value }) => validatePositiveNumber(value, "上昇速度"),
                }}
                children={(field) => (
                  <Field className="flex-1">
                    <FieldLabel className="text-xs" htmlFor={field.name}>上昇速度</FieldLabel>
                    <InputGroup>
                      <InputGroupInput
                        id={field.name}
                        name={field.name}
                        value={field.state.value}
                        onBlur={field.handleBlur}
                        onChange={(e) => field.handleChange(e.target.value)}
                        type="text"
                        placeholder="上昇速度"
                      />
                      <InputGroupAddon align="inline-start">
                        <ChevronsUp className="text-muted-foreground" />
                      </InputGroupAddon>
                    </InputGroup>
                    {field.state.meta.isTouched && field.state.meta.errors.length ? (
                      <p className="text-[11px] text-red-500 mt-1">{field.state.meta.errors.join(", ")}</p>
                    ) : null}
                  </Field>
                )}
              />

              {/* 落下速度 */}
              <form.Field
                name="descentRate"
                validators={{
                  onChange: ({ value }) => validatePositiveNumber(value, "落下速度"),
                }}
                children={(field) => (
                  <Field className="flex-1">
                    <FieldLabel className="text-xs" htmlFor={field.name}>落下速度</FieldLabel>
                    <InputGroup>
                      <InputGroupInput
                        id={field.name}
                        name={field.name}
                        value={field.state.value}
                        onBlur={field.handleBlur}
                        onChange={(e) => field.handleChange(e.target.value)}
                        type="text"
                        placeholder="落下速度"
                      />
                      <InputGroupAddon align="inline-start">
                        <ChevronsDown className="text-muted-foreground" />
                      </InputGroupAddon>
                    </InputGroup>
                    {field.state.meta.isTouched && field.state.meta.errors.length ? (
                      <p className="text-[11px] text-red-500 mt-1">{field.state.meta.errors.join(", ")}</p>
                    ) : null}
                  </Field>
                )}
              />
            </FieldGroup>

            {/* バースト高度 */}
            <form.Field
              name="burstAltitude"
              validators={{
                onChange: ({ value }) => validatePositiveNumber(value, "バースト高度"),
              }}
              children={(field) => (
                <Field>
                  <FieldLabel className="text-xs" htmlFor={field.name}>バースト高度</FieldLabel>
                  <InputGroup>
                    <InputGroupInput
                      id={field.name}
                      name={field.name}
                      value={field.state.value}
                      onBlur={field.handleBlur}
                      onChange={(e) => field.handleChange(e.target.value)}
                      type="text"
                      placeholder="高度を入力"
                    />
                    <InputGroupAddon align="inline-start">
                      <Loader className="text-muted-foreground" />
                    </InputGroupAddon>
                  </InputGroup>
                  {field.state.meta.isTouched && field.state.meta.errors.length ? (
                    <p className="text-[11px] text-red-500 mt-1">{field.state.meta.errors.join(", ")}</p>
                  ) : null}
                </Field>
              )}
            />
          </SidebarMenuItem>
        </SidebarContent>

        <SidebarFooter>
          <form.Subscribe
            selector={(state) => [state.canSubmit, state.isSubmitting]}
            children={([canSubmit, isSubmitting]) => (
              <Button type="submit" disabled={!canSubmit || isSubmitting || isLoading} className="w-full">
                {isSubmitting || isLoading ? (
                  <Loader className="animate-spin mr-2 h-4 w-4" />
                ) : (
                  <Play className="mr-2 h-4 w-4" />
                )}
                予測
              </Button>
            )}
          />
        </SidebarFooter>
      </form>
    </Sidebar>
  )
}