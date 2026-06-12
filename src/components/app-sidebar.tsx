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


export function AppSidebar() {
  return (
    <Sidebar variant="floating">
      <SidebarHeader className="py-4 px-3 flex flex-row">
        <Balloon />
        <h1>Space Balloon Predictor</h1>
      </SidebarHeader>
      <SidebarContent className="p-4 *:list-none">
        <SidebarMenuItem className="space-y-8">
          <Field>
            <FieldLabel className="text-xs" htmlFor="input-field-launch-position">放球位置</FieldLabel>
            <InputGroup>
              <InputGroupInput
                id="input-field-launch-position"
                type="text"
                placeholder="位置を入力"
              />
              <InputGroupAddon align="inline-start">
                <MapPin className="text-muted-foreground" />
              </InputGroupAddon>
            </InputGroup>
          </Field>
          <FieldGroup className="flex flex-row gap-x-2">
            <Field>
              <FieldLabel className="text-xs" htmlFor="input-field-launch-position">放球日</FieldLabel>
              <InputGroup>
                <InputGroupInput
                  id="input-field-launch-position"
                  type="text"
                  placeholder="日"
                />
                <InputGroupAddon align="inline-start">
                  <CalendarDays className="text-muted-foreground" />
                </InputGroupAddon>
              </InputGroup>
            </Field>
            <Field>
              <FieldLabel className="text-xs" htmlFor="input-field-launch-position">放球時刻</FieldLabel>
              <InputGroup>
                <InputGroupInput
                  id="input-field-launch-position"
                  type="text"
                  placeholder="時刻"
                />
                <InputGroupAddon align="inline-start">
                  <Clock className="text-muted-foreground" />
                </InputGroupAddon>
              </InputGroup>
            </Field>
          </FieldGroup>
          <FieldGroup className="flex flex-row gap-x-2">
            <Field>
              <FieldLabel className="text-xs" htmlFor="input-field-launch-position">上昇速度</FieldLabel>
              <InputGroup>
                <InputGroupInput
                  id="input-field-launch-position"
                  type="text"
                  placeholder="上昇速度"
                />
                <InputGroupAddon align="inline-start">
                  <ChevronsUp className="text-muted-foreground" />
                </InputGroupAddon>
              </InputGroup>
            </Field>
            <Field>
              <FieldLabel className="text-xs" htmlFor="input-field-launch-position">落下速度</FieldLabel>
              <InputGroup>
                <InputGroupInput
                  id="input-field-launch-position"
                  type="text"
                  placeholder="落下速度"
                />
                <InputGroupAddon align="inline-start">
                  <ChevronsDown className="text-muted-foreground" />
                </InputGroupAddon>
              </InputGroup>
            </Field>
          </FieldGroup>
          <Field>
            <FieldLabel className="text-xs" htmlFor="input-field-launch-position">バースト高度</FieldLabel>
            <InputGroup>
              <InputGroupInput
                id="input-field-launch-position"
                type="text"
                placeholder="位置を入力"
              />
              <InputGroupAddon align="inline-start">
                <Loader className="text-muted-foreground" />
              </InputGroupAddon>
            </InputGroup>
          </Field>
        </SidebarMenuItem>
      </SidebarContent>
      <SidebarFooter>
        <Button>
          <Play />
          予測
        </Button>
      </SidebarFooter>
    </Sidebar>
  )
}