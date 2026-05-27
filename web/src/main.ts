import Ical from 'ical.js';
import { Calendar as FullCalendar } from '@fullcalendar/core';
import dayGridPlugin from '@fullcalendar/daygrid';
import timeGridPlugin from '@fullcalendar/timegrid';
import interactionPlugin from '@fullcalendar/interaction';

const API_BASE = '/api';

interface ManagedCalendar {
  id: number;
  name: string;
  rapla_url: string;
  forked_ics_url: string;
}

interface Delta {
  id: string;
  calendar_id: number;
  type: 'delete' | 'modify' | 'add';
  match_key: string | null;
  event: OverlayEvent | null;
}

interface OverlayEvent {
  date: string;
  start: string;
  end: string;
  title: string;
  location?: string;
  organizer?: string;
  description?: string;
}

let calendars: ManagedCalendar[] = [];
let selectedCalendar: ManagedCalendar | null = null;
let calendar: FullCalendar | null = null;

async function api<T>(path: string, method = 'GET', body?: unknown): Promise<T> {
  const opts: RequestInit = {
    method,
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
  };
  if (body) opts.body = JSON.stringify(body);

  const res = await fetch(`${API_BASE}${path}`, opts);
  if (!res.ok) throw new Error(`${method} ${path}: ${res.status}`);
  if (res.status === 204) return null as T;
  return res.json();
}

async function fetchCalendars() {
  calendars = await api<ManagedCalendar[]>('/calendars');
  renderCalendarList();
}

function renderCalendarList() {
  const el = document.getElementById('calendars')!;
  el.innerHTML = '';
  for (const cal of calendars) {
    const li = document.createElement('li');
    if (selectedCalendar?.id === cal.id) li.classList.add('active');
    const publicUrl = `/public/${cal.id}`;
    li.innerHTML = `<span>${cal.name}</span><div class="cal-actions"><button class="public-view-btn" title="Public view">👁</button><button class="copy-link-btn" title="Copy ICS link">🔗</button><button class="delete-cal" data-id="${cal.id}">&times;</button></div>`;
    li.querySelector('span')!.addEventListener('click', () => selectCalendar(cal));
    li.querySelector('.public-view-btn')!.addEventListener('click', (e) => {
      e.stopPropagation();
      window.open(publicUrl, '_blank');
    });
    li.querySelector('.copy-link-btn')!.addEventListener('click', (e) => {
      e.stopPropagation();
      navigator.clipboard.writeText(cal.forked_ics_url);
      const btn = e.target as HTMLButtonElement;
      btn.textContent = '✓';
      setTimeout(() => { btn.textContent = '🔗'; }, 1500);
    });
    li.querySelector('.delete-cal')!.addEventListener('click', (e) => {
      e.stopPropagation();
      deleteCalendar(cal.id);
    });
    el.appendChild(li);
  }
}

async function selectCalendar(cal: ManagedCalendar) {
  selectedCalendar = cal;
  renderCalendarList();
  await loadCalendarEvents();
}

async function deleteCalendar(id: number) {
  await api(`/calendars/${id}`, 'DELETE');
  if (selectedCalendar?.id === id) selectedCalendar = null;
  await fetchCalendars();
}

function buildIcsUrl(cal: ManagedCalendar) {
  return cal.forked_ics_url;
}

async function loadCalendarEvents() {
  if (!selectedCalendar || !calendar) return;

  const icsUrl = buildIcsUrl(selectedCalendar);
  const res = await fetch(icsUrl);
  if (!res.ok) {
    calendar.removeAllEvents();
    return;
  }

  const text = await res.text();
  const jcalData = Ical.parse(text);
  const comp = new Ical.Component(jcalData);
  const events = comp.getAllSubcomponents('vevent');

  calendar.removeAllEvents();
  const fcEvents = events.map((vevent) => {
    const event = new Ical.Event(vevent);
    const isOverlay = event.summary.startsWith('[CM]') || event.summary.includes('[CM]');
    return {
      id: event.uid,
      title: event.summary,
      start: event.startDate.toJSDate(),
      end: event.endDate.toJSDate(),
      extendedProps: {
        isOverlay,
        matchKey: buildMatchKeyFromEvent(event),
        rawEvent: event,
      },
      classNames: isOverlay ? ['overlay-event'] : ['live-event'],
    };
  });

  calendar.addEventSource(fcEvents);
}

function buildMatchKeyFromEvent(event: Ical.Event): string {
  const start = event.startDate.toJSDate();
  const end = event.endDate.toJSDate();
  const pad = (n: number) => n.toString().padStart(2, '0');
  const date = `${start.getFullYear()}-${pad(start.getMonth() + 1)}-${pad(start.getDate())}`;
  const startTime = `${pad(start.getHours())}:${pad(start.getMinutes())}`;
  const endTime = `${pad(end.getHours())}:${pad(end.getMinutes())}`;
  return `${date}|${startTime}-${endTime}|${event.summary}`;
}

function buildMatchKeyFromDate(date: Date, startTime: string, endTime: string, title: string): string {
  const pad = (n: number) => n.toString().padStart(2, '0');
  const dateStr = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
  return `${dateStr}|${startTime}-${endTime}|${title}`;
}

function showModal(title: string, fields: { label: string; name: string; type: string; value?: string }[], onSubmit: (data: Record<string, string>) => void) {
  const overlay = document.getElementById('modal-overlay')!;
  const titleEl = document.getElementById('modal-title')!;
  const fieldsEl = document.getElementById('modal-fields')!;
  const actionsEl = document.getElementById('modal-actions')!;

  titleEl.textContent = title;
  fieldsEl.innerHTML = '';
  actionsEl.querySelectorAll('button:not(#modal-cancel):not(#modal-submit)').forEach(b => b.remove());

  for (const f of fields) {
    const label = document.createElement('label');
    label.innerHTML = `${f.label}<input type="${f.type}" name="${f.name}" value="${f.value ?? ''}" />`;
    fieldsEl.appendChild(label);
  }

  const form = document.getElementById('modal-form') as HTMLFormElement;
  const submitBtn = document.getElementById('modal-submit')!;
  const cancelBtn = document.getElementById('modal-cancel')!;

  const handler = (e: Event) => {
    e.preventDefault();
    const formData = new FormData(form);
    const data: Record<string, string> = {};
    for (const [key, value] of formData.entries()) {
      data[key] = value as string;
    }
    onSubmit(data);
    overlay.classList.add('hidden');
    form.removeEventListener('submit', handler);
    submitBtn.removeEventListener('click', handler as any);
    cancelBtn.removeEventListener('click', cancelHandler);
  };

  const cancelHandler = () => {
    overlay.classList.add('hidden');
    form.removeEventListener('submit', handler);
    submitBtn.removeEventListener('click', handler as any);
    cancelBtn.removeEventListener('click', cancelHandler);
  };

  form.addEventListener('submit', handler);
  cancelBtn.addEventListener('click', cancelHandler);
  overlay.classList.remove('hidden');
}

async function handleEventClick(info: any) {
  if (!selectedCalendar) return;

  const { isOverlay, matchKey, rawEvent } = info.event.extendedProps;
  const event = rawEvent as Ical.Event;

  const pad = (n: number) => n.toString().padStart(2, '0');
  const fmtDate = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  const fmtTime = (d: Date) => `${pad(d.getHours())}:${pad(d.getMinutes())}`;

  const startDate = event.startDate.toJSDate();
  const endDate = event.endDate.toJSDate();

  showModal(
    event.summary,
    [
      { label: 'Title', name: 'title', type: 'text', value: event.summary },
      { label: 'Date', name: 'date', type: 'date', value: fmtDate(startDate) },
      { label: 'Start', name: 'start', type: 'time', value: fmtTime(startDate) },
      { label: 'End', name: 'end', type: 'time', value: fmtTime(endDate) },
      { label: 'Location', name: 'location', type: 'text', value: event.location || '' },
      { label: 'Organizer', name: 'organizer', type: 'text', value: event.organizer || '' },
    ],
    async (data) => {
      const newEvent: OverlayEvent = {
        date: data.date,
        start: data.start,
        end: data.end,
        title: data.title,
        location: data.location || undefined,
        organizer: data.organizer || undefined,
      };

      await api<Delta>(`/calendars/${selectedCalendar!.id}/deltas`, 'POST', {
        type: 'modify',
        match_key: matchKey,
        event: newEvent,
      });

      await loadCalendarEvents();
    }
  );

  // Add delete button dynamically
  const actions = document.getElementById('modal-actions')!;
  const deleteBtn = document.createElement('button');
  deleteBtn.type = 'button';
  deleteBtn.textContent = 'Delete';
  deleteBtn.style.background = '#e94560';
  deleteBtn.style.color = '#fff';
  deleteBtn.style.marginRight = 'auto';
  deleteBtn.addEventListener('click', async () => {
    await api<Delta>(`/calendars/${selectedCalendar!.id}/deltas`, 'POST', {
      type: 'delete',
      match_key: matchKey,
    });
    document.getElementById('modal-overlay')!.classList.add('hidden');
    await loadCalendarEvents();
  });
  actions.prepend(deleteBtn);
}

function handleDateSelect(info: any) {
  if (!selectedCalendar) return;

  const pad = (n: number) => n.toString().padStart(2, '0');
  const fmtDate = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  const fmtTime = (d: Date) => `${pad(d.getHours())}:${pad(d.getMinutes())}`;

  showModal(
    'Add Event',
    [
      { label: 'Title', name: 'title', type: 'text' },
      { label: 'Date', name: 'date', type: 'date', value: fmtDate(info.start) },
      { label: 'Start', name: 'start', type: 'time', value: fmtTime(info.start) },
      { label: 'End', name: 'end', type: 'time', value: fmtTime(info.end) },
      { label: 'Location', name: 'location', type: 'text' },
      { label: 'Organizer', name: 'organizer', type: 'text' },
    ],
    async (data) => {
      const newEvent: OverlayEvent = {
        date: data.date,
        start: data.start,
        end: data.end,
        title: data.title,
        location: data.location || undefined,
        organizer: data.organizer || undefined,
      };

      await api<Delta>(`/calendars/${selectedCalendar!.id}/deltas`, 'POST', {
        type: 'add',
        event: newEvent,
      });

      await loadCalendarEvents();
    }
  );
}

document.getElementById('add-calendar-btn')!.addEventListener('click', () => {
  showModal(
    'Add Calendar',
    [
      { label: 'Name', name: 'name', type: 'text' },
      { label: 'Rapla URL', name: 'rapla_url', type: 'text' },
    ],
    async (data) => {
      await api('/calendars', 'POST', data);
      await fetchCalendars();
    }
  );
});

document.addEventListener('DOMContentLoaded', () => {
  const calendarEl = document.getElementById('calendar')!;
  calendar = new FullCalendar(calendarEl, {
    plugins: [dayGridPlugin, timeGridPlugin, interactionPlugin],
    initialView: 'timeGridWeek',
    headerToolbar: {
      left: 'prev,next today',
      center: 'title',
      right: 'dayGridMonth,timeGridWeek,timeGridDay',
    },
    selectable: true,
    editable: false,
    select: handleDateSelect,
    eventClick: handleEventClick,
  });
  calendar.render();

  fetchCalendars();
});
