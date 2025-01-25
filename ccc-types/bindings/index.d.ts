/* tslint:disable */

/* WARNING: This file is automatically generated */
/* DO NOT CHANGE IT MANUALLY */

export interface BonAppCafeResponse {
  name: string;
  address: string;
  city: string;
  state: string;
  zip: string;
  latitude: string;
  longitude: string;
  description: string;
  message: string;
  eod: string;
  timezone: string;
  menu_type: string;
  menu_html: string;
  location_detail: string;
  weekly_schedule: string;
  days: Array<BonAppDay>;
  cor_icons: CorIcons;
}

export interface BonAppCafesResponse {
  cafes: Record<string, BonAppCafeResponse>;
}

export interface BonAppCorIcon {
  sort: any;
  label: string;
  description: string;
  image: string;
  is_filter: YesNo;
  allergen: bigint;
}

export interface BonAppDay {
  date: string;
  dayparts: Array<BonAppDaypart>;
  status: string;
  message: string;
}

export interface BonAppDaypart {
  id: string;
  starttime: string;
  endtime: string;
  message: string | null;
  label: string;
  hide: string;
}

export interface BreakSchedule {
  fall: Array<Schedule>;
  thanksgiving: Array<Schedule>;
  winter: Array<Schedule>;
  interim: Array<Schedule>;
  spring: Array<Schedule>;
  easter: Array<Schedule>;
  summer: Array<Schedule>;
}

export interface BusTimesResponse {
  data: Array<BusTimesResponseItem>;
}

export interface BusTimesResponseItem {
  colors: TransitColors;
  line: string;
  notice: string | null;
  schedules: Array<TransitSchedule>;
}

export interface ColorPrintersResponse {
  data: ColorPrintersResponseItem;
}

export interface ColorPrintersResponseItem {
  colorPrinters: Array<string>;
}

export type Connector = string;

export interface ContactResponse {
  data: Array<ContactResponseItem>;
}

export interface ContactResponseItem {
  title: string;
  phoneNumber: string | null;
  buttonText: string | null;
  category: string;
  image?: string;
  synopsis: string;
  text: string;
}

export type CorIcons = Record<string, BonAppCorIcon>;

export type CurrencyString = string;

export type DayOfWeek = "Mo" | "Tu" | "We" | "Th" | "Fr" | "Sa" | "Su";

export interface DictionaryResponse {
  data: Array<DictionaryResponseItem>;
}

export interface DictionaryResponseItem {
  word: string;
  definition: string;
}

export interface FAQResponse {
  text: string;
}

export interface FoodItemResponse {
  label: string;
  station: string;
  description?: string;
  special?: boolean;
}

export interface FoodStationMenu {
  label: string;
  note?: string;
}

export interface HelpButton {
  title: string;
  action: string;
  params: HelpParams;
}

export interface HelpParams {
  url?: string;
  number?: string;
  to?: string;
  subject?: string;
  body?: string;
}

export interface HelpResponse {
  data: Array<HelpResponseItem>;
}

export interface HelpResponseItem {
  key: string;
  enabled: boolean;
  hidden: boolean;
  title: string;
  body: string;
  buttons: Array<HelpButton>;
}

export interface Hour {
  days: Array<DayOfWeek>;
  from: string;
  to: string;
}

export interface HoursItem {
  name: string;
  image?: string;
  category: string;
  schedule: Array<BreakSchedule>;
  breakSchedule: BreakSchedule;
  subtitle?: string;
}

export interface HoursResponse {
  data: Array<HoursItem>;
}

export type HtmlString = string;

export interface ItemNutritionDetails {
  calories?: NutritionDetail;
  servingSize?: NutritionDetail;
  fatContent?: NutritionDetail;
  saturatedFatContent?: NutritionDetail;
  transFatContent?: NutritionDetail;
  cholesterolContent?: NutritionDetail;
  sodiumContent?: NutritionDetail;
  carbohydrateContent?: NutritionDetail;
  fiberContent?: NutritionDetail;
  sugarContent?: NutritionDetail;
  proteinContent?: NutritionDetail;
}

export interface ModesItem {
  name: string;
  category: string;
  description: string;
  synopsis: string;
  url: string;
}

export interface ModesResponse {
  data: Array<ModesItem>;
}

export interface Monotony {
  id: string;
  name: string;
  short_name?: string;
  image: string;
}

export type MonotonyContainer = Monotony;

export type NumericString = string;

export interface Nutrition {
  kcal: NumericString;
  well_being: string;
  well_being_image: string;
}

export type NutritionContainer = Nutrition;

export interface NutritionDetail {
  label: string;
  value: number;
  unit: string;
}

export type NutritionDetailContainer = ItemNutritionDetails;

export type NutritionDetailType = NutritionDetail;

export interface PauseMenuItemResponse {
  stationMenus: Array<FoodStationMenu>;
  foodItems: Array<FoodItemResponse>;
  corIcons: Record<string, BonAppCorIcon>;
}

export interface PauseMenuResponse {
  data: PauseMenuItemResponse;
}

export interface Schedule {
  title: string;
  notes?: string;
  hours: Array<Hour>;
  closedForChapelTime: boolean | null;
  isPhysicallyOpen: boolean | null;
}

export type Stream = {
  starttime: string;
  location: string;
  eid: string;
  performer: string;
  subtitle: string;
  poster: string;
  player: string;
  status: string;
  category: string;
  hptitle: string;
  category_textcolor: string | null;
  category_color: string | null;
  thumb: string;
  title: string;
  iframesrc: string;
};

export type StreamResponse = { results: Array<Stream> };

export interface TransitColors {
  bar: string;
  dot: string;
}

export interface TransitSchedule {
  days: Array<string>;
  coordinates: Record<string, Array<number>>;
  stops: Array<string>;
  times: Array<Array<string | boolean>>;
}

export interface WebcamsResponse {
  data: Array<WebcamsResponseItem>;
}

export interface WebcamsResponseItem {
  name: string;
  pageUrl: string;
  streamUrl: string;
  thumbnail: string;
  thumbnailUrl?: string;
  tagline: string;
  accentColor: Array<bigint>;
  textColor: string;
}

export type YesNo = "N" | "Y";
