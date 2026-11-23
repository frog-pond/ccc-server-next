import {readFileSync} from 'node:fs'

if (process.permission?.has('net') !== false) {
    throw new Error('Run through tools/reference-fixture with Node.js 26 or newer to disable network access')
}

function fixture(path) {
    return readFileSync(new URL(path, import.meta.url), 'utf8')
}

async function athleticsScores() {
    const {scoresFromFeeds} = await import('./source/athletics/shape.ts')
    // Match workers/test/athletics.test.ts, including its shift to 2030.
    const recording = './source/athletics/fixtures/2026-09-26-home-games/20260926T180044Z'
    const scores = JSON.parse(fixture(recording + '-scores.json').replaceAll('2026', '2030'))
    const liveStats = JSON.parse(fixture(recording + '-livestats.json').replaceAll('2026', '2030'))
    return scoresFromFeeds(scores, liveStats, new Date('2030-09-26T18:00:44Z'))
}

async function carletonSumo() {
    const {default: moment} = await import('moment-timezone')
    const {carletonSpan, eventImages, withEventImages} = await import('./source/calendar/carleton-images-shape.ts')
    const {eventsFromIcal} = await import('./source/calendar/ical-shape.ts')
    // Match the Worker case's clock, fixture paths and year substitution.
    const fixtures = './workers/test/fixtures/'
    const page = 'https://www.carleton.edu/student/orgs/sumo/schedule/'
    const now = moment.utc('2030-10-09T12:00:00Z')
    const {maxEndDate} = carletonSpan(now)
    const ics = fixture(fixtures + 'carleton-sumo.ics').replaceAll('2026', '2030')
    const feed = page + '?loadFeed=calendar&stamp=1714840383'
    const events = eventsFromIcal(ics, feed, {maxEndDate}, now)
    const images = eventImages(fixture(fixtures + 'carleton-sumo-schedule.html'), page)
    return withEventImages(events, images)
}

const cases = new Map([
    ['athletics-scores', athleticsScores],
    ['carleton-sumo', carletonSumo],
])
const convert = cases.get(process.argv[2])
if (!convert) throw new Error('Unknown fixture case')
console.log(JSON.stringify(await convert(), null, 2))
