-- Initial schema for the Hospital Register.
--
-- Design decisions:
--
-- 1. UUIDs as primary keys. They prevent ID enumeration attacks and allow
--    clients to generate IDs before insert if we ever need that.
--
-- 2. TIMESTAMPTZ everywhere. Never TIMESTAMP (without time zone). A patient
--    registered in Kampala at 09:00 must be distinguishable from one
--    registered in London at 09:00.
--
-- 3. Soft deletes (deleted_at). Medical records must never truly vanish.
--    A `WHERE deleted_at IS NULL` filter gives us "active" rows.
--
-- 4. TEXT + CHECK instead of native ENUM types. Postgres enums are painful
--    to evolve. TEXT with a constraint gives us the same safety with trivial
--    migrations.
--
-- 5. Audit logs are append-only. No UPDATE, no DELETE.

-- pgcrypto provides gen_random_uuid(). In Postgres 13+ this is built in,
-- but we declare it explicitly for portability and clarity.
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ---------------------------------------------------------------------------
-- patients
-- ---------------------------------------------------------------------------
CREATE TABLE patients (
                          id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                          nin             VARCHAR(32) NOT NULL UNIQUE,
                          first_name      TEXT NOT NULL,
                          last_name       TEXT NOT NULL,
                          date_of_birth   DATE NOT NULL,
                          blood_type      TEXT CHECK (
                              blood_type IS NULL
                                  OR blood_type IN ('A+', 'A-', 'B+', 'B-', 'AB+', 'AB-', 'O+', 'O-')
                              ),
                          created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                          updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                          deleted_at      TIMESTAMPTZ
);

CREATE INDEX idx_patients_nin ON patients (nin) WHERE deleted_at IS NULL;
CREATE INDEX idx_patients_name ON patients (last_name, first_name) WHERE deleted_at IS NULL;

-- ---------------------------------------------------------------------------
-- doctors
-- ---------------------------------------------------------------------------
CREATE TABLE doctors (
                         id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                         email           TEXT NOT NULL UNIQUE,
                         password_hash   TEXT NOT NULL,
                         full_name       TEXT NOT NULL,
                         specialty       TEXT NOT NULL,
                         role            TEXT NOT NULL DEFAULT 'doctor' CHECK (
                             role IN ('doctor', 'admin', 'receptionist')
                             ),
                         created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                         updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                         deleted_at      TIMESTAMPTZ
);

CREATE INDEX idx_doctors_email ON doctors (email) WHERE deleted_at IS NULL;

-- ---------------------------------------------------------------------------
-- appointments
-- ---------------------------------------------------------------------------
CREATE TABLE appointments (
                              id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                              patient_id      UUID NOT NULL REFERENCES patients (id),
                              doctor_id       UUID NOT NULL REFERENCES doctors (id),
                              scheduled_for   TIMESTAMPTZ NOT NULL,
                              status          TEXT NOT NULL DEFAULT 'scheduled' CHECK (
                                  status IN ('scheduled', 'completed', 'cancelled', 'no_show')
                                  ),
                              notes           TEXT,
                              created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                              updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_appointments_patient ON appointments (patient_id);
CREATE INDEX idx_appointments_doctor_time ON appointments (doctor_id, scheduled_for);

-- ---------------------------------------------------------------------------
-- audit_logs
-- ---------------------------------------------------------------------------
-- `metadata` is JSONB, not JSON. JSONB is stored in a decomposed binary
-- form that is faster to query and supports indexing. JSON keeps the exact
-- text; JSONB normalizes it. For audit metadata we want queryability.
CREATE TABLE audit_logs (
                            id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                            actor_id        UUID,
                            action          TEXT NOT NULL,
                            entity_type     TEXT NOT NULL,
                            entity_id       UUID NOT NULL,
                            metadata        JSONB,
                            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_logs_entity ON audit_logs (entity_type, entity_id);
CREATE INDEX idx_audit_logs_actor ON audit_logs (actor_id);
CREATE INDEX idx_audit_logs_created ON audit_logs (created_at DESC);

-- ---------------------------------------------------------------------------
-- updated_at trigger
-- ---------------------------------------------------------------------------
-- We do not want application code to remember to set updated_at. A trigger
-- guarantees it. This is one of the places where using the database's own
-- features beats pure application logic.
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER patients_set_updated_at
    BEFORE UPDATE ON patients
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER doctors_set_updated_at
    BEFORE UPDATE ON doctors
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER appointments_set_updated_at
    BEFORE UPDATE ON appointments
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();