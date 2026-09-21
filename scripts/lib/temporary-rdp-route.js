// Supplemental route provisioning deliberately does not reconcile canonical A/B.
export function temporaryRoute(label) {
  if (!/^[C-Z]$/.test(label || '')) throw new Error('label must be C through Z; A/B are reserved');
  return { label, user: `agent-browser-rdp-${label.toLowerCase()}`,
    name: `Agent Browser RDP Temporary Route ${label}`, id: `guacamole-rdp-${label.toLowerCase()}` };
}

export function quote(value) {
  if (typeof value !== 'string' || /[\0\r\n\\]/.test(value)) throw new Error('invalid SQL value');
  return `'${value.replaceAll("'", "''")}'`;
}

export function selectedTemporaryLabel(args) {
  const index = args.indexOf('--route-label');
  if (index < 0) return null;
  return temporaryRoute(args[index + 1]).label;
}

export function selectTemporaryEntry(routes, label) {
  const expected = temporaryRoute(label);
  const selected = routes.filter(route => route.id === expected.id);
  if (selected.length !== 1 || selected[0].connectionName !== expected.name ||
      selected[0].target?.routeUser !== expected.user ||
      !/^[1-9][0-9]*$/.test(String(selected[0].connectionId)) ||
      selected[0].routeId !== `guacamole:${selected[0].connectionId}`) {
    throw new Error('temporary route selection missing, ambiguous or identity mismatched');
  }
  const route = selected[0];
  const localUrl = route.routeDescriptor?.localEmbedUrl || route.frameUrl;
  if (!localUrl) throw new Error('temporary route requires a loopback viewer URL');
  const local = new URL(localUrl);
  if (local.protocol !== 'http:' || !['127.0.0.1', '[::1]', 'localhost'].includes(local.hostname)) {
    throw new Error('temporary route viewer must use loopback Guacamole');
  }
  const urls = [route.frameUrl, route.externalUrl, ...['localEmbedUrl', 'dashboardEmbedUrl',
    'publicOperatorUrl', 'externalUrl', 'healthUrl'].map(key => route.routeDescriptor?.[key])].filter(Boolean);
  for (const value of urls) {
    const url = new URL(value);
    const match = /^#\/client\/([A-Za-z0-9+/=]+)$/.exec(url.hash);
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password ||
        url.pathname !== '/guacamole/' || url.search || !match ||
        Buffer.from(match[1], 'base64').toString('utf8') !== `${route.connectionId}\0c\0postgresql`) {
      throw new Error('temporary route viewer URL disagrees with connection identity');
    }
  }
  return selected;
}

export function connectionSql(route, password, operator) {
  if (!/^[a-f0-9]{64}$/.test(password)) throw new Error('invalid generated password');
  if (!/^[a-z_][a-z0-9_-]{0,31}$/.test(operator)) throw new Error('invalid operator');
  const params = { hostname: 'host.docker.internal', port: '3389', username: route.user,
    password, security: 'any', 'ignore-cert': 'true', 'resize-method': 'display-update',
    'enable-drive': 'false', 'enable-audio-input': 'false', 'disable-copy': 'true',
    'disable-paste': 'true' };
  return `BEGIN;
SELECT pg_advisory_xact_lock(174621903);
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM guacamole_connection WHERE connection_name = ${quote(route.name)}) THEN
    RAISE EXCEPTION 'temporary route name already exists';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM guacamole_entity WHERE name = ${quote(operator)} AND type = 'USER') THEN
    RAISE EXCEPTION 'operator Guacamole identity missing';
  END IF;
END $$;
WITH added AS (
  INSERT INTO guacamole_connection (connection_name, protocol, max_connections, max_connections_per_user)
  VALUES (${quote(route.name)}, 'rdp', 2, 2) RETURNING connection_id
), parameters AS (
  INSERT INTO guacamole_connection_parameter (connection_id, parameter_name, parameter_value)
  SELECT added.connection_id, p.name, p.value FROM added CROSS JOIN (VALUES
  ${Object.entries(params).map(([k,v]) => `(${quote(k)}, ${quote(v)})`).join(',\n  ')}
  ) AS p(name, value)
), permissions AS (
  INSERT INTO guacamole_connection_permission (entity_id, connection_id, permission)
  SELECT entity.entity_id, added.connection_id, 'READ'::guacamole_object_permission_type
  FROM added CROSS JOIN guacamole_entity entity WHERE entity.name = ${quote(operator)} AND entity.type = 'USER'
)
SELECT connection_id FROM added;
COMMIT;`;
}
