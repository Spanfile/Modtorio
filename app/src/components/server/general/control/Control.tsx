import React from 'react';
import { Row, Col, Button, Dropdown, SplitButton } from 'react-bootstrap';
import { ServerControlModal } from './ServerControlModal';
import { Wizard, Step } from 'components/common/Wizard';

import { ShutdownConfirmation } from './ShutdownConfirmation';
import { WaitPlayers } from './WaitPlayers';

export function Control(props: { id: string | undefined }) {
    const [modalShow, setModalShow] = React.useState(false);

    const handleClose = () => setModalShow(false);
    const handleShow = () => setModalShow(true);

    return (
        <>
            <Row xs={2} md={4} lg={6}>
                <Col className="pr-0">
                    <Button variant="outline-success" block disabled>Start</Button>
                </Col>
                <Col className="pr-0">
                    <SplitButton
                        key="shutdown"
                        id="shutdown"
                        variant="danger"
                        title="Shut down"
                        onClick={handleShow}
                    >
                        <Dropdown.Item eventKey="kill">Kill</Dropdown.Item>
                    </SplitButton>
                </Col>
                <Col className="pr-0">
                    <Button variant="warning" block>Restart</Button>
                </Col>
            </Row>
            <hr />
            <Row xs={2} md={4} lg={6}>
                <Col className="pr-0">
                    <Button variant="primary" block>Save</Button>
                </Col>
                <Col className="pr-0">
                    <Button variant="primary" block>Lua command</Button>
                </Col>
            </Row>

            <ServerControlModal variant="danger" onHide={handleClose} show={modalShow} title="Shut down">
                <Wizard done={handleClose}>
                    <Step render={(p) => <ShutdownConfirmation playerCount={1} {...p} />} />
                    <Step render={(p) => <WaitPlayers timeout={30} {...p} />} />
                </Wizard>
            </ServerControlModal>
        </>
    );
}
