import React from 'react';
import { Button } from 'react-bootstrap';

export type StepComponentProps<T extends { [K in keyof T]?: any } = {}> = {
    next: () => void,
    cancel: () => void,
    setUserControlled: (userControlled: boolean) => void,
    setCancelable: (cancelable: boolean) => void,
} & T

export type WizardProps = {
    initialStep?: number,
    done: (result: boolean) => void,
    children: React.ReactElement[];
}

export function Wizard(props: WizardProps) {
    const [step, setStep] = React.useState(props.initialStep ?? 0);

    let maxSteps = React.Children.count(props.children);
    let lastStep = maxSteps - 1;

    const success = () => props.done(true);
    const cancel = () => props.done(false);
    const nextStep = () => {
        if (step < lastStep) {
            return setStep(step + 1);
        } else {
            success();
        }
    };

    let steps = React.Children.map(
        props.children,
        (step) => React.cloneElement(step, { next: nextStep, cancel: cancel }));
    let currentStep = steps[step];

    const [nextEnabled, setNextEnabled] = React.useState(currentStep.props.userControlled ?? true);
    const [cancelEnabled, setCancelEnabled] = React.useState(currentStep.props.cancelable ?? true);

    return (
        <>
            {currentStep}
            <hr />
            <div className="d-flex flex-row-reverse">
                <Button
                    variant="primary"
                    className="ml-3"
                    onClick={nextStep}
                    disabled={!nextEnabled}>
                    {step < lastStep ? "Next" : "Done"}
                </Button>
                <Button
                    variant="secondary"
                    className="ml-3"
                    disabled={!cancelEnabled}
                    onClick={cancel}>
                    Cancel
                </Button>
            </div>
        </>
    );
}

type StepProps = {
    userControlled?: boolean,
    cancelable?: boolean,
    render: (props: StepComponentProps<any>) => React.ReactNode;
}

export function Step(props: StepProps) {
    return (
        <>
            {props.render({ ...props })}
        </>
    );
}
